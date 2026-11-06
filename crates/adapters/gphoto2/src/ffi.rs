// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Hand-written declarations of the parts of the libgphoto2 C API (`gphoto2.h`) and of
//! `src/shim.c` that the adapter uses (ADR 0034 §3).
//!
//! Functions return a libgphoto2 result code: [`OK`] or more on success, a negative
//! `GP_ERROR_*` code on failure. Functions without preconditions are declared `safe`;
//! the others need valid pointers from the matching `*_new` function that are not
//! freed yet.

use std::ffi::{c_char, c_int};

/// `GP_OK`.
pub(crate) const OK: c_int = 0;
/// `GP_ERROR_BAD_PARAMETERS`.
pub(crate) const ERROR_BAD_PARAMETERS: c_int = -2;
/// `GP_ERROR_NO_MEMORY`.
pub(crate) const ERROR_NO_MEMORY: c_int = -3;
/// `GP_ERROR_NOT_SUPPORTED`.
pub(crate) const ERROR_NOT_SUPPORTED: c_int = -6;
/// `GP_ERROR_IO_USB_CLAIM`: another program has claimed the USB device.
pub(crate) const ERROR_IO_USB_CLAIM: c_int = -53;
/// `GP_ERROR_IO_LOCK`: another program has locked the port.
pub(crate) const ERROR_IO_LOCK: c_int = -60;
/// `GP_ERROR_DIRECTORY_NOT_FOUND`.
pub(crate) const ERROR_DIRECTORY_NOT_FOUND: c_int = -107;
/// `GP_ERROR_FILE_NOT_FOUND`.
pub(crate) const ERROR_FILE_NOT_FOUND: c_int = -108;
/// `GP_ERROR_CAMERA_BUSY`.
pub(crate) const ERROR_CAMERA_BUSY: c_int = -110;

/// The opaque `Camera`.
#[repr(C)]
pub(crate) struct Camera {
    _private: [u8; 0],
}

/// The opaque `GPContext`.
#[repr(C)]
pub(crate) struct Context {
    _private: [u8; 0],
}

/// The opaque `CameraList`: pairs of a name and a value.
#[repr(C)]
pub(crate) struct List {
    _private: [u8; 0],
}

// SAFETY: the signatures match the headers of the libgphoto2 version the flake pins and
// src/shim.c; build.rs links the libraries that define them.
unsafe extern "C" {
    /// Returns a new context, or null if it cannot be allocated.
    pub(crate) safe fn gp_context_new() -> *mut Context;
    pub(crate) fn gp_context_unref(context: *mut Context);
    pub(crate) fn gp_list_new(list: *mut *mut List) -> c_int;
    pub(crate) fn gp_list_free(list: *mut List) -> c_int;
    pub(crate) fn gp_list_count(list: *mut List) -> c_int;
    pub(crate) fn gp_list_get_name(
        list: *mut List,
        index: c_int,
        name: *mut *const c_char,
    ) -> c_int;
    pub(crate) fn gp_list_get_value(
        list: *mut List,
        index: c_int,
        value: *mut *const c_char,
    ) -> c_int;
    /// Fills `list` with the model names and ports of the connected cameras.
    pub(crate) fn gp_camera_autodetect(list: *mut List, context: *mut Context) -> c_int;
    pub(crate) fn gp_camera_new(camera: *mut *mut Camera) -> c_int;
    pub(crate) fn gp_camera_init(camera: *mut Camera, context: *mut Context) -> c_int;
    pub(crate) fn gp_camera_exit(camera: *mut Camera, context: *mut Context) -> c_int;
    pub(crate) fn gp_camera_unref(camera: *mut Camera) -> c_int;
    pub(crate) fn gp_camera_folder_list_files(
        camera: *mut Camera,
        folder: *const c_char,
        list: *mut List,
        context: *mut Context,
    ) -> c_int;
    pub(crate) fn gp_camera_folder_list_folders(
        camera: *mut Camera,
        folder: *const c_char,
        list: *mut List,
        context: *mut Context,
    ) -> c_int;
    /// Returns a static, NUL-terminated message for every code.
    pub(crate) safe fn gp_result_as_string(result: c_int) -> *const c_char;
    pub(crate) fn framehive_gp_select(
        camera: *mut Camera,
        model: *const c_char,
        port: *const c_char,
    ) -> c_int;
    pub(crate) fn framehive_gp_file_info(
        camera: *mut Camera,
        folder: *const c_char,
        file: *const c_char,
        size: *mut u64,
        has_size: *mut c_int,
        modification_time: *mut i64,
        has_modification_time: *mut c_int,
        context: *mut Context,
    ) -> c_int;
    pub(crate) fn framehive_gp_file_read(
        camera: *mut Camera,
        folder: *const c_char,
        file: *const c_char,
        offset: u64,
        buffer: *mut c_char,
        size: *mut u64,
        context: *mut Context,
    ) -> c_int;
    pub(crate) fn framehive_gp_forget_listings(camera: *mut Camera) -> c_int;
}
