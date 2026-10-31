// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Builders for JPEG, TIFF and RAF test files with EXIF text tags.
//!
//! Tests assemble their files in code, so no binary file enters the repository
//! (ADR 0021, decision 5). The files hold only what the capture time needs: a TIFF
//! header, IFD0 with the EXIF IFD pointer, the EXIF IFD and the text values.

/// EXIF tag `DateTime` (IFD0): when the file was last changed, not a capture time.
pub const DATE_TIME: u16 = 0x0132;
/// EXIF tag `DateTimeOriginal` (EXIF IFD): the capture time.
pub const DATE_TIME_ORIGINAL: u16 = 0x9003;
/// EXIF tag `SubSecTimeOriginal` (EXIF IFD): the fraction of a second.
pub const SUB_SEC_TIME_ORIGINAL: u16 = 0x9291;
/// EXIF tag `OffsetTimeOriginal` (EXIF IFD): the UTC offset.
pub const OFFSET_TIME_ORIGINAL: u16 = 0x9011;

const EXIF_IFD_POINTER: u16 = 0x8769;
const TYPE_ASCII: u16 = 2;
const TYPE_LONG: u16 = 4;
const HEADER_LENGTH: u32 = 8;
const ENTRY_LENGTH: u32 = 12;

/// The byte order of a TIFF structure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ByteOrder {
    /// `II`, least significant byte first.
    LittleEndian,
    /// `MM`, most significant byte first.
    BigEndian,
}

/// A TIFF structure with text tags in IFD0 and in the EXIF IFD.
///
/// ```
/// use metadata::capture_time_from_bytes;
/// use metadata::testing::{ByteOrder, DATE_TIME_ORIGINAL, Tiff};
///
/// let raw = Tiff::new(ByteOrder::BigEndian)
///     .with_exif_text(DATE_TIME_ORIGINAL, "2026:10:17 14:05:09")
///     .to_bytes();
/// let time = capture_time_from_bytes(&raw)?;
/// assert_eq!(time.date_time().hour(), 14);
/// # Ok::<(), metadata::NoCaptureTime>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tiff {
    order: ByteOrder,
    magic: u16,
    ifd0_texts: Vec<(u16, String)>,
    exif_texts: Vec<(u16, String)>,
    exif_ifd_offset: Option<u32>,
}

impl Tiff {
    /// An empty TIFF with magic 42 and an empty EXIF IFD.
    #[must_use]
    pub fn new(order: ByteOrder) -> Self {
        Tiff {
            order,
            magic: 42,
            ifd0_texts: Vec::new(),
            exif_texts: Vec::new(),
            exif_ifd_offset: None,
        }
    }

    /// Uses another magic number, such as `0x4F52` for ORF or `0x0055` for RW2.
    #[must_use]
    pub fn with_magic(mut self, magic: u16) -> Self {
        self.magic = magic;
        self
    }

    /// Adds an ASCII tag to IFD0.
    #[must_use]
    pub fn with_ifd0_text(mut self, tag: u16, text: &str) -> Self {
        self.ifd0_texts.push((tag, text.to_owned()));
        self
    }

    /// Adds an ASCII tag to the EXIF IFD.
    #[must_use]
    pub fn with_exif_text(mut self, tag: u16, text: &str) -> Self {
        self.exif_texts.push((tag, text.to_owned()));
        self
    }

    /// Points the EXIF IFD pointer at `offset` instead of the EXIF IFD, e.g. at IFD0
    /// (offset 8) for a loop.
    #[must_use]
    pub fn with_exif_ifd_pointer(mut self, offset: u32) -> Self {
        self.exif_ifd_offset = Some(offset);
        self
    }

    /// The bytes: header, IFD0, EXIF IFD, then the values longer than four bytes.
    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        let ifd0_count = len_u32(self.ifd0_texts.len()) + 1;
        let exif_ifd = HEADER_LENGTH + ifd_length(ifd0_count);
        let values_start = exif_ifd + ifd_length(len_u32(self.exif_texts.len()));
        let mut values = Vec::new();
        let mut ifd0 = self.text_entries(&self.ifd0_texts, values_start, &mut values);
        let pointer = self.exif_ifd_offset.unwrap_or(exif_ifd);
        ifd0.push(self.entry(EXIF_IFD_POINTER, TYPE_LONG, 1, self.u32(pointer)));
        let exif = self.text_entries(&self.exif_texts, values_start, &mut values);

        let mut bytes = match self.order {
            ByteOrder::LittleEndian => b"II".to_vec(),
            ByteOrder::BigEndian => b"MM".to_vec(),
        };
        bytes.extend(self.u16(self.magic));
        bytes.extend(self.u32(HEADER_LENGTH));
        self.write_ifd(&mut bytes, ifd0);
        self.write_ifd(&mut bytes, exif);
        bytes.extend(values);
        bytes
    }

    fn text_entries(
        &self,
        texts: &[(u16, String)],
        values_start: u32,
        values: &mut Vec<u8>,
    ) -> Vec<[u8; 12]> {
        texts
            .iter()
            .map(|(tag, text)| {
                let mut value = text.as_bytes().to_vec();
                value.push(0);
                let count = len_u32(value.len());
                let field = if value.len() <= 4 {
                    value.resize(4, 0);
                    [value[0], value[1], value[2], value[3]]
                } else {
                    let offset = values_start + len_u32(values.len());
                    values.extend(&value);
                    self.u32(offset)
                };
                self.entry(*tag, TYPE_ASCII, count, field)
            })
            .collect()
    }

    fn entry(&self, tag: u16, kind: u16, count: u32, value: [u8; 4]) -> [u8; 12] {
        let mut entry = [0; 12];
        entry[0..2].copy_from_slice(&self.u16(tag));
        entry[2..4].copy_from_slice(&self.u16(kind));
        entry[4..8].copy_from_slice(&self.u32(count));
        entry[8..12].copy_from_slice(&value);
        entry
    }

    /// Writes the entries sorted by tag, as TIFF requires, and no next IFD.
    fn write_ifd(&self, bytes: &mut Vec<u8>, mut entries: Vec<[u8; 12]>) {
        entries.sort_by_key(|entry| self.read_u16([entry[0], entry[1]]));
        let count = u16::try_from(entries.len()).unwrap_or(u16::MAX);
        bytes.extend(self.u16(count));
        for entry in entries {
            bytes.extend(entry);
        }
        bytes.extend(self.u32(0));
    }

    fn u16(&self, value: u16) -> [u8; 2] {
        match self.order {
            ByteOrder::LittleEndian => value.to_le_bytes(),
            ByteOrder::BigEndian => value.to_be_bytes(),
        }
    }

    fn u32(&self, value: u32) -> [u8; 4] {
        match self.order {
            ByteOrder::LittleEndian => value.to_le_bytes(),
            ByteOrder::BigEndian => value.to_be_bytes(),
        }
    }

    fn read_u16(&self, bytes: [u8; 2]) -> u16 {
        match self.order {
            ByteOrder::LittleEndian => u16::from_le_bytes(bytes),
            ByteOrder::BigEndian => u16::from_be_bytes(bytes),
        }
    }
}

fn ifd_length(entries: u32) -> u32 {
    2 + entries * ENTRY_LENGTH + 4
}

/// Test files stay far below 4 GiB.
fn len_u32(length: usize) -> u32 {
    u32::try_from(length).unwrap_or(u32::MAX)
}

/// A JPEG whose `APP1` segment holds `tiff` as its EXIF, after a JFIF `APP0` and an
/// XMP `APP1` segment, followed by a little image data.
///
/// # Panics
///
/// Panics if `tiff` does not fit in one segment (64 KiB).
#[must_use]
pub fn jpeg(tiff: &[u8]) -> Vec<u8> {
    let mut bytes = vec![0xFF, 0xD8];
    segment(&mut bytes, 0xE0, b"JFIF\0\x01\x02\0\0\x01\0\x01\0\0");
    segment(
        &mut bytes,
        0xE1,
        b"http://ns.adobe.com/xap/1.0/\0<x:xmpmeta/>",
    );
    let mut exif = b"Exif\0\0".to_vec();
    exif.extend(tiff);
    segment(&mut bytes, 0xE1, &exif);
    segment(&mut bytes, 0xDA, b"\x01\x01\x00\x00\x3F\x00");
    bytes.extend([0x12, 0x34, 0x56, 0xFF, 0xD9]);
    bytes
}

fn segment(bytes: &mut Vec<u8>, marker: u8, payload: &[u8]) {
    #[allow(clippy::expect_used, reason = "a documented panic of a test builder")]
    let length = u16::try_from(payload.len() + 2).expect("the segment fits in 64 KiB");
    bytes.extend([0xFF, marker]);
    bytes.extend(length.to_be_bytes());
    bytes.extend(payload);
}

/// A RAF whose header points at `embedded_jpeg`, which follows the header.
#[must_use]
pub fn raf(embedded_jpeg: &[u8]) -> Vec<u8> {
    const HEADER_LENGTH: usize = 160;
    let mut bytes = b"FUJIFILMCCD-RAW 0201FF383501".to_vec();
    bytes.extend(b"X-T5");
    bytes.resize(HEADER_LENGTH, 0);
    let offset = len_u32(HEADER_LENGTH);
    bytes[84..88].copy_from_slice(&offset.to_be_bytes());
    bytes[88..92].copy_from_slice(&len_u32(embedded_jpeg.len()).to_be_bytes());
    bytes.extend(embedded_jpeg);
    bytes
}
