// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Safe wrappers of libgphoto2's result codes, contexts, lists and autodetection.

use std::ffi::{CStr, CString, OsString, c_char, c_int};
use std::os::unix::ffi::OsStringExt;
use std::ptr;
use std::sync::{Mutex, MutexGuard, PoisonError};

use crate::ffi;

/// Loading and unloading camera drivers changes global state of libgphoto2 and libltdl
/// that is not safe to change from two threads at once.
static DRIVERS: Mutex<()> = Mutex::new(());

/// Holds the lock on loading and unloading camera drivers.
pub(crate) fn lock_drivers() -> MutexGuard<'static, ()> {
    // The lock guards no data, so a panic while it was held left nothing inconsistent.
    DRIVERS.lock().unwrap_or_else(PoisonError::into_inner)
}

/// A failed libgphoto2 call: its negative result code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct GpError(pub(crate) c_int);

impl GpError {
    /// libgphoto2's message for the code.
    pub(crate) fn message(self) -> String {
        let text = ffi::gp_result_as_string(self.0);
        if text.is_null() {
            return format!("libgphoto2 error {}", self.0);
        }
        // SAFETY: gp_result_as_string returns a static, NUL-terminated string.
        let text = unsafe { CStr::from_ptr(text) };
        format!("{} ({})", text.to_string_lossy(), self.0)
    }

    /// Another program holds the camera, such as gvfs or `ptpcamerad` (ADR 0034 §6).
    pub(crate) fn is_busy(self) -> bool {
        matches!(
            self.0,
            ffi::ERROR_IO_USB_CLAIM | ffi::ERROR_IO_LOCK | ffi::ERROR_CAMERA_BUSY
        )
    }

    /// The folder or file does not exist, or libgphoto2 has not listed it yet.
    pub(crate) fn is_not_found(self) -> bool {
        matches!(
            self.0,
            ffi::ERROR_DIRECTORY_NOT_FOUND | ffi::ERROR_FILE_NOT_FOUND
        )
    }

    /// The driver cannot do what was asked, for example read part of a file.
    pub(crate) fn is_not_supported(self) -> bool {
        self.0 == ffi::ERROR_NOT_SUPPORTED
    }
}

/// Turns a libgphoto2 result code into a result.
pub(crate) fn check(result: c_int) -> Result<c_int, GpError> {
    if result >= ffi::OK {
        Ok(result)
    } else {
        Err(GpError(result))
    }
}

/// Copies `text` into a C string; text with a NUL byte cannot name anything on a
/// camera.
pub(crate) fn c_string(text: impl Into<Vec<u8>>) -> Result<CString, GpError> {
    CString::new(text).map_err(|_| GpError(ffi::ERROR_BAD_PARAMETERS))
}

/// A `GPContext`, unreferenced when dropped.
pub(crate) struct Context(*mut ffi::Context);

impl Context {
    pub(crate) fn new() -> Result<Self, GpError> {
        let context = ffi::gp_context_new();
        if context.is_null() {
            Err(GpError(ffi::ERROR_NO_MEMORY))
        } else {
            Ok(Self(context))
        }
    }

    pub(crate) fn as_ptr(&self) -> *mut ffi::Context {
        self.0
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        // SAFETY: the context comes from gp_context_new and is not used afterwards.
        unsafe { ffi::gp_context_unref(self.0) };
    }
}

/// A `CameraList`, freed when dropped.
pub(crate) struct List(*mut ffi::List);

impl List {
    pub(crate) fn new() -> Result<Self, GpError> {
        let mut list = ptr::null_mut();
        // SAFETY: gp_list_new writes a new list to the valid pointer it gets.
        check(unsafe { ffi::gp_list_new(&raw mut list) })?;
        Ok(Self(list))
    }

    pub(crate) fn as_ptr(&self) -> *mut ffi::List {
        self.0
    }

    /// The names of the entries, such as the names of files in a folder.
    pub(crate) fn names(&self) -> Result<Vec<OsString>, GpError> {
        (0..self.count()?).map(|index| self.name(index)).collect()
    }

    /// The names and values of the entries, such as models and ports.
    pub(crate) fn pairs(&self) -> Result<Vec<(OsString, OsString)>, GpError> {
        (0..self.count()?)
            .map(|index| Ok((self.name(index)?, self.value(index)?)))
            .collect()
    }

    fn count(&self) -> Result<c_int, GpError> {
        // SAFETY: the list is valid until it is dropped.
        check(unsafe { ffi::gp_list_count(self.0) })
    }

    fn name(&self, index: c_int) -> Result<OsString, GpError> {
        let mut name = ptr::null();
        // SAFETY: the list is valid, the index is below its count, and the pointer is
        // valid for one write.
        check(unsafe { ffi::gp_list_get_name(self.0, index, &raw mut name) })?;
        // SAFETY: the list owns the string until it is changed or freed, which this
        // borrow of the list prevents.
        Ok(unsafe { copy_text(name) })
    }

    fn value(&self, index: c_int) -> Result<OsString, GpError> {
        let mut value = ptr::null();
        // SAFETY: as in `name`.
        check(unsafe { ffi::gp_list_get_value(self.0, index, &raw mut value) })?;
        // SAFETY: as in `name`.
        Ok(unsafe { copy_text(value) })
    }
}

impl Drop for List {
    fn drop(&mut self) {
        // SAFETY: the list comes from gp_list_new and is not used afterwards.
        unsafe { ffi::gp_list_free(self.0) };
    }
}

/// Copies a string libgphoto2 returned; null is the empty string.
///
/// # Safety
///
/// `text` is null or points to a NUL-terminated string that stays valid during the
/// call.
unsafe fn copy_text(text: *const c_char) -> OsString {
    if text.is_null() {
        return OsString::new();
    }
    // SAFETY: the caller guarantees a valid, NUL-terminated string.
    let text = unsafe { CStr::from_ptr(text) };
    OsString::from_vec(text.to_bytes().to_vec())
}

/// The model names and ports of the connected cameras, as libgphoto2 detects them.
pub(crate) fn autodetect() -> Result<Vec<(String, String)>, GpError> {
    let _drivers = lock_drivers();
    let context = Context::new()?;
    let list = List::new()?;
    // SAFETY: list and context are valid until they are dropped at the end of the call.
    check(unsafe { ffi::gp_camera_autodetect(list.as_ptr(), context.as_ptr()) })?;
    Ok(list
        .pairs()?
        .into_iter()
        .map(|(model, port)| {
            (
                model.to_string_lossy().into_owned(),
                port.to_string_lossy().into_owned(),
            )
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_message_names_the_code() {
        assert!(GpError(-108).message().ends_with("(-108)"));
    }
}
