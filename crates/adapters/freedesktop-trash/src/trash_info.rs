// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The `.trashinfo` file and the names of trashed files.

use std::ffi::{OsStr, OsString};
use std::fmt::Write;
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::path::Path;

use domain::LocalDateTime;

/// The content of the `.trashinfo` file for a file trashed from `original` at
/// `deleted`, in local time without a time zone, as the specification asks.
pub(crate) fn content(original: &Path, deleted: LocalDateTime) -> String {
    format!(
        "[Trash Info]\nPath={}\nDeletionDate={deleted}\n",
        percent_encoded(original)
    )
}

/// `path` with every byte except the unreserved characters of RFC 2396 and `/`
/// written as `%XX`.
fn percent_encoded(path: &Path) -> String {
    let mut encoded = String::new();
    for &byte in path.as_os_str().as_bytes() {
        if byte.is_ascii_alphanumeric() || b"-_.!~*'()/".contains(&byte) {
            encoded.push(char::from(byte));
        } else {
            let _infallible = write!(encoded, "%{byte:02X}");
        }
    }
    encoded
}

/// The name of a trashed file: `file_name` itself first, then with a number before
/// its extension, `DSCF0001.2.RAF`.
pub(crate) fn numbered_name(file_name: &OsStr, number: u32) -> OsString {
    if number == 1 {
        return file_name.to_os_string();
    }
    let path = Path::new(file_name);
    let mut name = path.file_stem().unwrap_or(file_name).as_bytes().to_vec();
    name.extend_from_slice(format!(".{number}").as_bytes());
    if let Some(extension) = path.extension() {
        name.push(b'.');
        name.extend_from_slice(extension.as_bytes());
    }
    OsString::from_vec(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_content_names_the_path_and_the_local_deletion_time() {
        let deleted = LocalDateTime::new(2026, 11, 9, 18, 5, 7).unwrap();

        let content = content(Path::new("/home/a/2026-11-09/DSCF0001.RAF"), deleted);

        assert_eq!(
            content,
            "[Trash Info]\nPath=/home/a/2026-11-09/DSCF0001.RAF\nDeletionDate=2026-11-09T18:05:07\n"
        );
    }

    #[test]
    fn spaces_percent_signs_and_non_ascii_bytes_are_percent_encoded() {
        let encoded = percent_encoded(Path::new("/Fotos/Zürich 100%/A.JPG"));

        assert_eq!(encoded, "/Fotos/Z%C3%BCrich%20100%25/A.JPG");
    }

    #[test]
    fn the_first_name_is_the_file_name() {
        assert_eq!(numbered_name(OsStr::new("DSCF0001.RAF"), 1), "DSCF0001.RAF");
    }

    #[test]
    fn later_names_have_the_number_before_the_extension() {
        assert_eq!(
            numbered_name(OsStr::new("DSCF0001.RAF.xmp"), 2),
            "DSCF0001.RAF.2.xmp"
        );
        assert_eq!(numbered_name(OsStr::new("README"), 3), "README.3");
    }
}
