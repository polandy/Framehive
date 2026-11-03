// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Hand-written declarations of the parts of the LibRaw C API (`libraw.h`), of
//! `src/shim.c` and of the OpenMP runtime that the decoder uses (ADR 0012 §1).
//!
//! Functions without preconditions are declared `safe`; the others need a valid
//! `libraw_data_t` from [`libraw_init`] that is not closed yet.

use std::ffi::{c_char, c_int, c_uint, c_ushort, c_void};

/// `LIBRAW_SUCCESS`.
pub(crate) const SUCCESS: c_int = 0;
/// `LIBRAW_FILE_UNSUPPORTED`.
pub(crate) const FILE_UNSUPPORTED: c_int = -2;
/// `LIBRAW_DATA_ERROR`.
pub(crate) const DATA_ERROR: c_int = -100_008;
/// `LIBRAW_IO_ERROR`: LibRaw reads from a buffer, so this means the data ended early.
pub(crate) const IO_ERROR: c_int = -100_009;
/// `LIBRAW_IMAGE_BITMAP` of `enum LibRaw_image_formats`.
pub(crate) const IMAGE_BITMAP: c_int = 2;
/// `output_color` 1: sRGB.
pub(crate) const OUTPUT_COLOR_SRGB: c_int = 1;

/// The opaque `libraw_data_t`.
#[repr(C)]
pub(crate) struct LibRawData {
    _private: [u8; 0],
}

/// `libraw_processed_image_t`; `data` is the first of `data_size` bytes.
#[repr(C)]
pub(crate) struct ProcessedImage {
    /// `enum LibRaw_image_formats`, which C stores as an `int`.
    pub(crate) kind: c_int,
    pub(crate) height: c_ushort,
    pub(crate) width: c_ushort,
    pub(crate) colors: c_ushort,
    pub(crate) bits: c_ushort,
    pub(crate) data_size: c_uint,
    pub(crate) data: [u8; 1],
}

// SAFETY: the signatures match libraw.h of the LibRaw version the flake pins, src/shim.c
// and omp.h; build.rs links the libraries that define them.
unsafe extern "C" {
    /// Returns a new `libraw_data_t`, or null if it cannot be allocated.
    pub(crate) safe fn libraw_init(flags: c_uint) -> *mut LibRawData;
    pub(crate) fn libraw_open_buffer(
        data: *mut LibRawData,
        buffer: *const c_void,
        size: usize,
    ) -> c_int;
    pub(crate) fn libraw_unpack(data: *mut LibRawData) -> c_int;
    pub(crate) fn libraw_dcraw_process(data: *mut LibRawData) -> c_int;
    pub(crate) fn libraw_dcraw_make_mem_image(
        data: *mut LibRawData,
        error: *mut c_int,
    ) -> *mut ProcessedImage;
    pub(crate) fn libraw_dcraw_clear_mem(image: *mut ProcessedImage);
    pub(crate) fn libraw_close(data: *mut LibRawData);
    /// Returns a static, NUL-terminated message for every code.
    pub(crate) safe fn libraw_strerror(code: c_int) -> *const c_char;
    #[link_name = "libraw_versionNumber"]
    pub(crate) safe fn libraw_version_number() -> c_int;
    pub(crate) fn libraw_set_output_bps(data: *mut LibRawData, value: c_int);
    pub(crate) fn libraw_set_output_color(data: *mut LibRawData, value: c_int);
    pub(crate) fn libraw_set_no_auto_bright(data: *mut LibRawData, value: c_int);
    pub(crate) fn framehive_libraw_set_use_camera_wb(data: *mut LibRawData, value: c_int);
    pub(crate) fn framehive_libraw_set_user_flip(data: *mut LibRawData, value: c_int);
    /// The number of threads the next parallel region of the calling thread uses.
    pub(crate) safe fn omp_get_max_threads() -> c_int;
    /// Sets that number for the calling thread only.
    pub(crate) safe fn omp_set_num_threads(threads: c_int);
}
