// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The [`LibRawDecoder`].

use std::ffi::{CStr, c_int};
use std::fs;
use std::marker::PhantomData;
use std::path::Path;
use std::ptr::NonNull;
use std::{mem, slice};

use previews::{DecodeError, DecodeMode, DecodedImage, PreviewDecoder};

use crate::ffi;

/// Bits per sample of the decoded image (PREVIEWS-001).
const BITS_PER_SAMPLE: c_int = 16;
/// Samples per pixel of the decoded image.
const CHANNELS: u16 = 3;

/// A [`PreviewDecoder`] that decodes RAW files with LibRaw (ADR 0012).
///
/// Both modes decode to 16-bit sRGB with the camera's white balance, no automatic
/// brightening, LibRaw's default demosaicing and no rotation; the reference mode uses
/// one OpenMP thread (ADR 0028 §3).
#[derive(Debug, Default, Clone, Copy)]
pub struct LibRawDecoder;

impl LibRawDecoder {
    /// A decoder; it holds no state, every decode opens its own LibRaw instance.
    #[must_use]
    pub fn new() -> Self {
        LibRawDecoder
    }
}

impl PreviewDecoder for LibRawDecoder {
    fn version(&self) -> String {
        let number = ffi::libraw_version_number();
        format!(
            "LibRaw {}.{}.{}",
            number >> 16,
            (number >> 8) & 0xff,
            number & 0xff
        )
    }

    fn decode_full(&self, path: &Path, mode: DecodeMode) -> Result<DecodedImage, DecodeError> {
        let file = fs::read(path).map_err(|source| DecodeError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        let _threads = ThreadLimit::for_mode(mode);
        decode(&file).map_err(|failure| failure.into_error(path))
    }
}

/// Why LibRaw did not return an image.
enum Failure {
    /// A LibRaw error code.
    Code(c_int),
    /// Anything else, with a message.
    Other(String),
}

impl Failure {
    fn into_error(self, path: &Path) -> DecodeError {
        let path = path.to_path_buf();
        match self {
            Failure::Code(ffi::FILE_UNSUPPORTED) => DecodeError::UnsupportedFormat { path },
            Failure::Code(code @ (ffi::DATA_ERROR | ffi::IO_ERROR)) => DecodeError::Corrupt {
                path,
                message: message(code),
            },
            Failure::Code(code) => DecodeError::Failed {
                path,
                message: message(code),
            },
            Failure::Other(message) => DecodeError::Failed { path, message },
        }
    }
}

/// LibRaw's message for `code`.
fn message(code: c_int) -> String {
    let text = ffi::libraw_strerror(code);
    if text.is_null() {
        return format!("LibRaw error {code}");
    }
    // SAFETY: libraw_strerror returns a static, NUL-terminated string for every code.
    unsafe { CStr::from_ptr(text) }
        .to_string_lossy()
        .into_owned()
}

fn checked(code: c_int) -> Result<(), Failure> {
    if code == ffi::SUCCESS {
        Ok(())
    } else {
        Err(Failure::Code(code))
    }
}

fn decode(file: &[u8]) -> Result<DecodedImage, Failure> {
    let processor = Processor::open(file)?;
    processor.configure();
    processor.process()?;
    processor.image()?.to_decoded()
}

/// Limits OpenMP to one thread on the calling thread for a reference decode, and
/// restores the previous limit when dropped.
struct ThreadLimit {
    previous: Option<c_int>,
}

impl ThreadLimit {
    fn for_mode(mode: DecodeMode) -> Self {
        let previous = match mode {
            DecodeMode::Reference => {
                let previous = ffi::omp_get_max_threads();
                ffi::omp_set_num_threads(1);
                Some(previous)
            }
            DecodeMode::Interactive => None,
        };
        ThreadLimit { previous }
    }
}

impl Drop for ThreadLimit {
    fn drop(&mut self) {
        if let Some(previous) = self.previous {
            ffi::omp_set_num_threads(previous);
        }
    }
}

/// A LibRaw instance that has opened `file`, which it reads without copying.
struct Processor<'file> {
    data: NonNull<ffi::LibRawData>,
    file: PhantomData<&'file [u8]>,
}

impl<'file> Processor<'file> {
    fn open(file: &'file [u8]) -> Result<Self, Failure> {
        let data = NonNull::new(ffi::libraw_init(0))
            .ok_or_else(|| Failure::Other("cannot allocate a LibRaw instance".to_owned()))?;
        let processor = Processor {
            data,
            file: PhantomData,
        };
        // SAFETY: `data` is a live instance; `file` outlives the processor through its
        // lifetime, so LibRaw may keep reading it until libraw_close.
        checked(unsafe {
            ffi::libraw_open_buffer(data.as_ptr(), file.as_ptr().cast(), file.len())
        })?;
        Ok(processor)
    }

    /// Sets the decode configuration of PREVIEWS-001; the defaults cover the
    /// demosaicing.
    fn configure(&self) {
        let data = self.data.as_ptr();
        // SAFETY: `data` is a live instance (this applies to each call below).
        unsafe { ffi::libraw_set_output_bps(data, BITS_PER_SAMPLE) };
        // SAFETY: as above.
        unsafe { ffi::libraw_set_output_color(data, ffi::OUTPUT_COLOR_SRGB) };
        // SAFETY: as above.
        unsafe { ffi::libraw_set_no_auto_bright(data, 1) };
        // SAFETY: as above.
        unsafe { ffi::framehive_libraw_set_use_camera_wb(data, 1) };
        // SAFETY: as above.
        unsafe { ffi::framehive_libraw_set_user_flip(data, 0) };
    }

    fn process(&self) -> Result<(), Failure> {
        // SAFETY: `data` is a live instance with an opened file.
        checked(unsafe { ffi::libraw_unpack(self.data.as_ptr()) })?;
        // SAFETY: `data` is a live instance whose RAW data was unpacked.
        checked(unsafe { ffi::libraw_dcraw_process(self.data.as_ptr()) })
    }

    fn image(&self) -> Result<MemoryImage, Failure> {
        let mut code = ffi::SUCCESS;
        // SAFETY: `data` is a live, processed instance and `code` a valid int.
        let image = unsafe { ffi::libraw_dcraw_make_mem_image(self.data.as_ptr(), &raw mut code) };
        checked(code)?;
        NonNull::new(image)
            .map(MemoryImage)
            .ok_or_else(|| Failure::Other("LibRaw returned no image".to_owned()))
    }
}

impl Drop for Processor<'_> {
    fn drop(&mut self) {
        // SAFETY: `data` came from libraw_init and is closed only here.
        unsafe { ffi::libraw_close(self.data.as_ptr()) };
    }
}

/// An image from `libraw_dcraw_make_mem_image`, freed when dropped.
struct MemoryImage(NonNull<ffi::ProcessedImage>);

impl MemoryImage {
    fn to_decoded(&self) -> Result<DecodedImage, Failure> {
        // SAFETY: the pointer came from libraw_dcraw_make_mem_image and is freed only
        // when `self` drops.
        let header = unsafe { self.0.as_ref() };
        if header.kind != ffi::IMAGE_BITMAP || header.colors != CHANNELS || header.bits != 16 {
            return Err(Failure::Other(format!(
                "LibRaw returned an image of type {} with {} colours and {} bits",
                header.kind, header.colors, header.bits
            )));
        }
        let length =
            usize::try_from(header.data_size).map_err(|error| Failure::Other(error.to_string()))?;
        // From the raw pointer, not from `header`, whose borrow covers one byte of `data`.
        let start = self
            .0
            .as_ptr()
            .cast::<u8>()
            .wrapping_add(mem::offset_of!(ffi::ProcessedImage, data));
        // SAFETY: LibRaw allocates `data_size` bytes starting at `data`, which stay
        // valid while `self` lives.
        let bytes = unsafe { slice::from_raw_parts(start, length) };
        let samples = bytes
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| u16::from_ne_bytes(*pair))
            .collect();
        DecodedImage::new(header.width.into(), header.height.into(), samples)
            .map_err(|error| Failure::Other(error.to_string()))
    }
}

impl Drop for MemoryImage {
    fn drop(&mut self) {
        // SAFETY: the pointer came from libraw_dcraw_make_mem_image and is freed only
        // here.
        unsafe { ffi::libraw_dcraw_clear_mem(self.0.as_ptr()) };
    }
}
