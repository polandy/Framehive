// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The TIFF structure of EXIF (CIPA DC-008 §4.5): a header, then IFDs of 12-byte
//! entries. Only IFD0 and the EXIF IFD it points to are read.

use domain::CaptureTime;

use crate::NoCaptureTime;
use crate::source::{Failure, Reader, Window, damaged};
use crate::values;

const HEADER_LENGTH: usize = 8;
const ENTRY_LENGTH: usize = 12;
/// More entries than any camera writes; it bounds what a damaged count makes us read.
const MAX_ENTRIES_PER_IFD: u16 = 4_096;
/// The longest text value read; a date and time needs 20 bytes.
const MAX_TEXT_LENGTH: u32 = 64;

/// The magic numbers after the byte order: 42 for TIFF, and the variants of Olympus
/// ORF (`RO`, `RS`) and Panasonic RW2 (`U`).
const MAGIC_NUMBERS: [u16; 4] = [42, 0x4F52, 0x5352, 0x0055];

const EXIF_IFD_POINTER: u16 = 0x8769;
const DATE_TIME_ORIGINAL: u16 = 0x9003;
const SUB_SEC_TIME_ORIGINAL: u16 = 0x9291;
const OFFSET_TIME_ORIGINAL: u16 = 0x9011;

const TYPE_BYTE: u16 = 1;
const TYPE_ASCII: u16 = 2;
const TYPE_LONG: u16 = 4;
const TYPE_UNDEFINED: u16 = 7;
const TYPE_IFD: u16 = 13;

/// Whether `header` starts like a TIFF file in either byte order.
pub(crate) fn is_tiff(header: &[u8]) -> bool {
    match header {
        [b'I', b'I', low, high, ..] => MAGIC_NUMBERS.contains(&u16::from_le_bytes([*low, *high])),
        [b'M', b'M', high, low, ..] => MAGIC_NUMBERS.contains(&u16::from_be_bytes([*high, *low])),
        _ => false,
    }
}

/// The capture time in the TIFF data that fills `window`.
pub(crate) fn capture_time(
    reader: &mut Reader<'_>,
    window: Window,
) -> Result<CaptureTime, Failure> {
    let header = reader.read(window, 0, HEADER_LENGTH, "TIFF header")?;
    if !is_tiff(&header) {
        return Err(damaged("the TIFF header is not valid"));
    }
    let mut tiff = Tiff {
        reader,
        window,
        order: if header[0] == b'I' {
            ByteOrder::Little
        } else {
            ByteOrder::Big
        },
        visited: Vec::new(),
    };
    let ifd0 = tiff.order.u32(&header[4..8]);
    let exif_pointer = tiff
        .entries(ifd0)?
        .into_iter()
        .find(|entry| entry.tag == EXIF_IFD_POINTER)
        .ok_or(NoCaptureTime::Missing)?;
    let exif_entries = tiff.entries(tiff.pointer(&exif_pointer)?)?;
    let find = |tag| exif_entries.iter().find(|entry| entry.tag == tag);
    let date_time = find(DATE_TIME_ORIGINAL).ok_or(NoCaptureTime::Missing)?;
    let date_time = tiff.text(date_time)?;
    let sub_seconds = tiff.optional_text(find(SUB_SEC_TIME_ORIGINAL))?;
    let offset = tiff.optional_text(find(OFFSET_TIME_ORIGINAL))?;
    Ok(values::capture_time(
        &date_time,
        sub_seconds.as_deref(),
        offset.as_deref(),
    )?)
}

#[derive(Debug, Clone, Copy)]
enum ByteOrder {
    Little,
    Big,
}

impl ByteOrder {
    fn u16(self, bytes: &[u8]) -> u16 {
        let pair = [bytes[0], bytes[1]];
        match self {
            ByteOrder::Little => u16::from_le_bytes(pair),
            ByteOrder::Big => u16::from_be_bytes(pair),
        }
    }

    fn u32(self, bytes: &[u8]) -> u32 {
        let quad = [bytes[0], bytes[1], bytes[2], bytes[3]];
        match self {
            ByteOrder::Little => u32::from_le_bytes(quad),
            ByteOrder::Big => u32::from_be_bytes(quad),
        }
    }
}

/// One 12-byte IFD entry; `value` holds the value itself if it fits in four bytes,
/// and its offset otherwise.
#[derive(Debug, Clone, Copy)]
struct Entry {
    tag: u16,
    kind: u16,
    count: u32,
    value: [u8; 4],
}

struct Tiff<'r, 's> {
    reader: &'r mut Reader<'s>,
    window: Window,
    order: ByteOrder,
    /// The offsets of the IFDs read so far, so crafted pointers cannot loop.
    visited: Vec<u32>,
}

impl Tiff<'_, '_> {
    /// The entries of the IFD at `offset`.
    fn entries(&mut self, offset: u32) -> Result<Vec<Entry>, Failure> {
        if self.visited.contains(&offset) {
            return Err(damaged(format!("IFD at offset {offset} is read twice")));
        }
        self.visited.push(offset);
        let count_bytes = self.reader.read(self.window, offset.into(), 2, "IFD")?;
        let count = self.order.u16(&count_bytes);
        if count > MAX_ENTRIES_PER_IFD {
            return Err(damaged(format!(
                "IFD at offset {offset} has {count} entries"
            )));
        }
        let length = usize::from(count) * ENTRY_LENGTH;
        let bytes = self
            .reader
            .read(self.window, u64::from(offset) + 2, length, "IFD entries")?;
        Ok(bytes
            .as_chunks::<ENTRY_LENGTH>()
            .0
            .iter()
            .map(|entry| Entry {
                tag: self.order.u16(&entry[0..2]),
                kind: self.order.u16(&entry[2..4]),
                count: self.order.u32(&entry[4..8]),
                value: [entry[8], entry[9], entry[10], entry[11]],
            })
            .collect())
    }

    /// The offset an IFD pointer entry holds.
    fn pointer(&self, entry: &Entry) -> Result<u32, Failure> {
        if !matches!(entry.kind, TYPE_LONG | TYPE_IFD) || entry.count != 1 {
            return Err(damaged(format!(
                "IFD pointer {:#06x} has type {} and count {}",
                entry.tag, entry.kind, entry.count
            )));
        }
        Ok(self.order.u32(&entry.value))
    }

    /// The bytes of a text value, at most [`MAX_TEXT_LENGTH`] of them.
    fn text(&mut self, entry: &Entry) -> Result<Vec<u8>, Failure> {
        if !matches!(entry.kind, TYPE_BYTE | TYPE_ASCII | TYPE_UNDEFINED) {
            return Err(Failure::Reason(NoCaptureTime::Invalid {
                value: format!("a value of type {}", entry.kind),
            }));
        }
        let length = entry.count.min(MAX_TEXT_LENGTH);
        if entry.count <= 4 {
            return Ok(entry.value[..length as usize].to_vec());
        }
        let offset = self.order.u32(&entry.value);
        let what = format!("value of tag {:#06x}", entry.tag);
        self.reader
            .read(self.window, offset.into(), length as usize, &what)
    }

    /// The text of an optional tag; a damaged value is ignored like an invalid one,
    /// because the date and time alone decide the capture time.
    fn optional_text(&mut self, entry: Option<&Entry>) -> Result<Option<Vec<u8>>, Failure> {
        match entry.map(|entry| self.text(entry)) {
            None | Some(Err(Failure::Reason(_))) => Ok(None),
            Some(Ok(text)) => Ok(Some(text)),
            Some(Err(failure)) => Err(failure),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognises_tiff_and_its_raw_variants_in_both_byte_orders() {
        assert!(is_tiff(b"II*\0"));
        assert!(is_tiff(b"MM\0*"));
        assert!(is_tiff(b"IIRO"));
        assert!(is_tiff(b"IIRS"));
        assert!(is_tiff(b"MMOR"));
        assert!(is_tiff(b"IIU\0"));
        assert!(!is_tiff(b"II\0*"));
        assert!(!is_tiff(b"MM*\0"));
        assert!(!is_tiff(b"II*"));
    }
}
