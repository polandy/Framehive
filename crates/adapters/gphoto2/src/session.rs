// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! [`Session`]: one opened libgphoto2 camera.

use std::ffi::{CStr, OsString, c_int};
use std::ptr;

use crate::ffi;
use crate::library::{Context, GpError, List, c_string, check, lock_drivers};

/// What libgphoto2 reports about a file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FileInfo {
    /// The size in bytes, if the driver reports it.
    pub(crate) size: Option<u64>,
    /// The modification time in seconds since the Unix epoch, if the driver reports it.
    pub(crate) modified: Option<i64>,
}

/// An opened libgphoto2 camera, closed and released when dropped.
///
/// A libgphoto2 camera must not be used from two threads at once; every method takes
/// `&mut self`, and the file system shares the session behind a mutex.
pub(crate) struct Session {
    camera: *mut ffi::Camera,
    context: Context,
}

// SAFETY: a libgphoto2 camera and its context are not bound to the thread that created
// them; `&mut self` on every call keeps two threads from using them at once.
unsafe impl Send for Session {}

impl Session {
    /// Opens the camera of `model` at `port`.
    pub(crate) fn open(model: &str, port: &str) -> Result<Self, GpError> {
        let model = c_string(model)?;
        let port = c_string(port)?;
        let _drivers = lock_drivers();
        let context = Context::new()?;
        let mut camera = ptr::null_mut();
        // SAFETY: gp_camera_new writes a new camera to the valid pointer it gets.
        check(unsafe { ffi::gp_camera_new(&raw mut camera) })?;
        if let Err(error) = select_and_init(camera, &model, &port, &context) {
            // SAFETY: the camera comes from gp_camera_new and is not used afterwards;
            // gp_camera_unref also undoes a partial gp_camera_init.
            unsafe { ffi::gp_camera_unref(camera) };
            return Err(error);
        }
        Ok(Self { camera, context })
    }

    /// The names of the folders in `folder`.
    pub(crate) fn list_folders(&mut self, folder: &CStr) -> Result<Vec<OsString>, GpError> {
        let list = List::new()?;
        // SAFETY: camera, list and context are valid; folder is NUL-terminated.
        check(unsafe {
            ffi::gp_camera_folder_list_folders(
                self.camera,
                folder.as_ptr(),
                list.as_ptr(),
                self.context.as_ptr(),
            )
        })?;
        list.names()
    }

    /// The names of the files in `folder`.
    pub(crate) fn list_files(&mut self, folder: &CStr) -> Result<Vec<OsString>, GpError> {
        let list = List::new()?;
        // SAFETY: camera, list and context are valid; folder is NUL-terminated.
        check(unsafe {
            ffi::gp_camera_folder_list_files(
                self.camera,
                folder.as_ptr(),
                list.as_ptr(),
                self.context.as_ptr(),
            )
        })?;
        list.names()
    }

    /// What libgphoto2 reports about the file `name` in `folder`.
    pub(crate) fn file_info(&mut self, folder: &CStr, name: &CStr) -> Result<FileInfo, GpError> {
        let mut size = 0;
        let mut has_size: c_int = 0;
        let mut modified = 0;
        let mut has_modified: c_int = 0;
        // SAFETY: camera and context are valid, the strings are NUL-terminated, and
        // every output pointer is valid for one write.
        check(unsafe {
            ffi::framehive_gp_file_info(
                self.camera,
                folder.as_ptr(),
                name.as_ptr(),
                &raw mut size,
                &raw mut has_size,
                &raw mut modified,
                &raw mut has_modified,
                self.context.as_ptr(),
            )
        })?;
        Ok(FileInfo {
            size: (has_size != 0).then_some(size),
            modified: (has_modified != 0).then_some(modified),
        })
    }

    /// Reads the file `name` in `folder` from `offset` into `buffer`, and returns the
    /// number of bytes read; it is less than the buffer only at the end of the file.
    pub(crate) fn read(
        &mut self,
        folder: &CStr,
        name: &CStr,
        offset: u64,
        buffer: &mut [u8],
    ) -> Result<usize, GpError> {
        let mut size = buffer.len() as u64;
        // SAFETY: camera and context are valid, the strings are NUL-terminated, and
        // the driver writes at most `size` bytes to the buffer.
        check(unsafe {
            ffi::framehive_gp_file_read(
                self.camera,
                folder.as_ptr(),
                name.as_ptr(),
                offset,
                buffer.as_mut_ptr().cast(),
                &raw mut size,
                self.context.as_ptr(),
            )
        })?;
        // A driver never reports more than it was asked for; the minimum guards the
        // slice the caller takes from the buffer.
        Ok(usize::try_from(size).map_or(buffer.len(), |size| size.min(buffer.len())))
    }

    /// Runs `operation`; if it finds no folder or file, forgets what libgphoto2 has
    /// listed and runs it once more.
    ///
    /// libgphoto2 keeps every listing it has made, so a folder or file added to the
    /// camera after its parent folder was listed is not found until the listings are
    /// forgotten. Forgetting them only on a miss keeps the scan of a camera fast.
    pub(crate) fn with_fresh_listings<T>(
        &mut self,
        mut operation: impl FnMut(&mut Self) -> Result<T, GpError>,
    ) -> Result<T, GpError> {
        match operation(self) {
            Err(error) if error.is_not_found() => {
                // SAFETY: the camera is valid until the session is dropped.
                check(unsafe { ffi::framehive_gp_forget_listings(self.camera) })?;
                operation(self)
            }
            result => result,
        }
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        let _drivers = lock_drivers();
        // SAFETY: camera and context are valid; a failure to close leaves nothing to
        // undo, because the camera is released next either way.
        unsafe { ffi::gp_camera_exit(self.camera, self.context.as_ptr()) };
        // SAFETY: the camera comes from gp_camera_new and is not used afterwards.
        unsafe { ffi::gp_camera_unref(self.camera) };
    }
}

/// Makes `camera` the camera of `model` at `port` and opens it.
fn select_and_init(
    camera: *mut ffi::Camera,
    model: &CStr,
    port: &CStr,
    context: &Context,
) -> Result<(), GpError> {
    // SAFETY: the camera comes from gp_camera_new; the strings are NUL-terminated.
    check(unsafe { ffi::framehive_gp_select(camera, model.as_ptr(), port.as_ptr()) })?;
    // SAFETY: the camera and the context are valid.
    check(unsafe { ffi::gp_camera_init(camera, context.as_ptr()) })?;
    Ok(())
}
