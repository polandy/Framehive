// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Reading the Linux mount table, `/proc/self/mountinfo` (proc(5)).

use std::ffi::OsString;
use std::path::PathBuf;

/// One mount of the mount table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Mount {
    /// Where the file system is mounted.
    pub(crate) mount_point: PathBuf,
    /// What is mounted, such as `/dev/sdb1`.
    pub(crate) source: OsString,
}

/// The mounts of a `mountinfo` text, in its order. Lines that do not have the form
/// of proc(5) are skipped.
pub(crate) fn parse(text: &[u8]) -> Vec<Mount> {
    text.split(|byte| *byte == b'\n')
        .filter_map(parse_line)
        .collect()
}

/// A line is `id parent major:minor root mount-point options [optional…] - type
/// source super-options`; fields are separated by single spaces, and spaces in
/// fields are escaped as octal.
fn parse_line(line: &[u8]) -> Option<Mount> {
    let fields: Vec<&[u8]> = line.split(|byte| *byte == b' ').collect();
    let mount_point = fields.get(4)?;
    let separator = fields.iter().skip(6).position(|field| *field == b"-")? + 6;
    let source = fields.get(separator + 2)?;
    if !mount_point.starts_with(b"/") {
        return None;
    }
    Some(Mount {
        mount_point: PathBuf::from(os_string(unescape(mount_point))),
        source: os_string(unescape(source)),
    })
}

/// Decodes the octal escapes of the kernel, such as `\040` for a space; any other
/// backslash is kept as it is.
fn unescape(field: &[u8]) -> Vec<u8> {
    let mut decoded = Vec::with_capacity(field.len());
    let mut index = 0;
    while index < field.len() {
        if let Some(byte) = octal_escape(&field[index..]) {
            decoded.push(byte);
            index += 4;
        } else {
            decoded.push(field[index]);
            index += 1;
        }
    }
    decoded
}

/// The byte of an escape `\ooo` at the start of `rest`.
fn octal_escape(rest: &[u8]) -> Option<u8> {
    let [b'\\', digits @ ..] = rest.get(..4)? else {
        return None;
    };
    let mut value: u32 = 0;
    for digit in digits {
        if !(b'0'..=b'7').contains(digit) {
            return None;
        }
        value = value * 8 + u32::from(digit - b'0');
    }
    u8::try_from(value).ok()
}

#[cfg(unix)]
fn os_string(bytes: Vec<u8>) -> OsString {
    use std::os::unix::ffi::OsStringExt;
    OsString::from_vec(bytes)
}

/// Framehive runs on Linux and macOS (ADR 0023); elsewhere only valid UTF-8 survives.
#[cfg(not(unix))]
fn os_string(bytes: Vec<u8>) -> OsString {
    OsString::from(String::from_utf8_lossy(&bytes).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    const CARD_LINE: &[u8] = b"512 29 8:17 / /run/media/anna/EOS_DIGITAL rw,nosuid,nodev,relatime shared:279 - vfat /dev/sdb1 rw,uid=1000";

    #[test]
    fn a_line_gives_the_mount_point_and_the_source() {
        assert_eq!(
            parse(CARD_LINE),
            [Mount {
                mount_point: PathBuf::from("/run/media/anna/EOS_DIGITAL"),
                source: OsString::from("/dev/sdb1"),
            }]
        );
    }

    #[test]
    fn optional_fields_may_be_missing_or_many() {
        let text = b"22 1 0:21 / /media/a/X rw - vfat /dev/sdc1 rw\n\
                     23 1 0:22 / /media/a/Y rw shared:1 master:2 propagate_from:3 - exfat /dev/sdd1 rw\n";
        let sources: Vec<OsString> = parse(text).into_iter().map(|mount| mount.source).collect();
        assert_eq!(sources, ["/dev/sdc1", "/dev/sdd1"].map(OsString::from));
    }

    /// spec: DEVICES-001.5
    #[test]
    fn octal_escapes_are_decoded() {
        let text = br"40 29 8:17 / /run/media/anna/NO\040NAME rw - vfat /dev/sdb1 rw";
        assert_eq!(
            parse(text)[0].mount_point,
            PathBuf::from("/run/media/anna/NO NAME")
        );
        assert_eq!(unescape(br"a\011b\134c\012"), b"a\tb\\c\n");
    }

    #[test]
    fn a_backslash_without_three_octal_digits_is_kept() {
        assert_eq!(unescape(br"a\08b\0"), br"a\08b\0");
    }

    #[test]
    fn malformed_lines_are_skipped() {
        let text =
            b"\nnot a mount\n1 2 3 4 relative rw - vfat /dev/sdb1 rw\n1 2 3 / /media/a/X rw\n";
        assert_eq!(parse(text), []);
    }
}
