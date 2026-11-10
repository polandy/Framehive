// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The Objective-C runtime and CoreFoundation calls behind [`trash_item`]. NSURL and
//! NSError are toll-free bridged to CFURL and CFError, so only two messages are sent.

use std::ffi::{CStr, OsStr, c_char, c_void};
use std::io;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::ptr;

type Object = *mut c_void;
type Selector = *const c_void;
type CfIndex = isize;

type DefaultManagerFn = unsafe extern "C" fn(Object, Selector) -> Object;
type TrashItemFn = unsafe extern "C" fn(Object, Selector, Object, *mut Object, *mut Object) -> i8;

const UTF8: u32 = 0x0800_0100;
/// More than macOS's `PATH_MAX` of 1024 bytes.
const PATH_BUFFER: usize = 4096;
const COCOA_ERROR_DOMAIN: &str = "NSCocoaErrorDomain";

#[link(name = "objc")]
unsafe extern "C" {
    safe fn objc_autoreleasePoolPush() -> *mut c_void;
    fn objc_autoreleasePoolPop(pool: *mut c_void);
    fn objc_getClass(name: *const c_char) -> Object;
    fn sel_registerName(name: *const c_char) -> Selector;
    /// Called only through a pointer of the exact type of the method it sends.
    fn objc_msgSend();
}

#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    fn CFURLCreateFromFileSystemRepresentation(
        allocator: *const c_void,
        buffer: *const u8,
        length: CfIndex,
        is_directory: u8,
    ) -> Object;
    fn CFURLGetFileSystemRepresentation(
        url: Object,
        resolve_against_base: u8,
        buffer: *mut u8,
        max_length: CfIndex,
    ) -> u8;
    fn CFErrorGetCode(error: Object) -> CfIndex;
    fn CFErrorGetDomain(error: Object) -> Object;
    fn CFErrorCopyDescription(error: Object) -> Object;
    fn CFStringGetLength(string: Object) -> CfIndex;
    safe fn CFStringGetMaximumSizeForEncoding(length: CfIndex, encoding: u32) -> CfIndex;
    fn CFStringGetCString(string: Object, buffer: *mut c_char, size: CfIndex, encoding: u32) -> u8;
    fn CFRelease(object: Object);
}

// Registers the NSFileManager class with the runtime.
#[link(name = "Foundation", kind = "framework")]
unsafe extern "C" {}

/// Moves the file at the absolute `path` to the Trash and returns where it lies there.
pub(crate) fn trash_item(path: &Path) -> io::Result<PathBuf> {
    let _pool = AutoreleasePool::push();
    let url = Owned::file_url(path)?;
    let manager = default_file_manager()?;
    let selector = selector(c"trashItemAtURL:resultingItemURL:error:");
    let mut resulting: Object = ptr::null_mut();
    let mut error: Object = ptr::null_mut();
    // SAFETY: `TrashItemFn` is the exact type of the method: the receiver, the
    // selector, an NSURL, and two out pointers, returning a BOOL.
    let trash_item = unsafe { message::<TrashItemFn>() };
    // SAFETY: `manager` is the shared NSFileManager, `url` a valid file URL that lives
    // until the call returns, and both out pointers are writable; the objects written
    // there are autoreleased into `_pool`, which outlives every use below.
    let moved = unsafe { trash_item(manager, selector, url.0, &raw mut resulting, &raw mut error) };
    if moved == 0 || resulting.is_null() {
        return Err(io_error(error));
    }
    path_of(resulting)
}

/// `objc_msgSend` as a pointer of the method type `F`.
///
/// # Safety
///
/// `F` must be an `unsafe extern "C" fn` with the exact argument and return types of
/// the method it is used to send.
unsafe fn message<F: Copy>() -> F {
    let send: unsafe extern "C" fn() = objc_msgSend;
    // SAFETY: both are function pointers of the same size; the caller promises that
    // `F` matches the method, which is how `objc_msgSend` must be called.
    unsafe { std::mem::transmute_copy::<unsafe extern "C" fn(), F>(&send) }
}

fn selector(name: &CStr) -> Selector {
    // SAFETY: `name` is NUL-terminated; the runtime copies it.
    unsafe { sel_registerName(name.as_ptr()) }
}

fn default_file_manager() -> io::Result<Object> {
    // SAFETY: the name is NUL-terminated; the runtime only reads it.
    let class = unsafe { objc_getClass(c"NSFileManager".as_ptr()) };
    if class.is_null() {
        return Err(io::Error::other("NSFileManager is not available"));
    }
    // SAFETY: `+defaultManager` takes no arguments and returns an object.
    let default_manager = unsafe { message::<DefaultManagerFn>() };
    // SAFETY: `class` is the NSFileManager class, which answers `+defaultManager`.
    let manager = unsafe { default_manager(class, selector(c"defaultManager")) };
    if manager.is_null() {
        return Err(io::Error::other("NSFileManager has no default manager"));
    }
    Ok(manager)
}

/// The path of the file URL `url`.
fn path_of(url: Object) -> io::Result<PathBuf> {
    let mut buffer = vec![0_u8; PATH_BUFFER];
    let length = CfIndex::try_from(buffer.len()).map_err(io::Error::other)?;
    // SAFETY: `url` is a valid CFURL and `buffer` is writable for `length` bytes.
    let filled = unsafe { CFURLGetFileSystemRepresentation(url, 1, buffer.as_mut_ptr(), length) };
    let path = CStr::from_bytes_until_nul(&buffer)
        .ok()
        .filter(|_| filled != 0);
    let path = path.ok_or_else(|| io::Error::other("the Trash returned no path"))?;
    Ok(PathBuf::from(OsStr::from_bytes(path.to_bytes())))
}

/// The I/O error for the NSError `error`, which may be null.
fn io_error(error: Object) -> io::Error {
    if error.is_null() {
        return io::Error::other("the Trash did not take the file");
    }
    // SAFETY: `error` is a valid CFError; the domain is not owned by the caller.
    let domain = unsafe { CFErrorGetDomain(error) };
    // SAFETY: `error` is a valid CFError.
    let code = unsafe { CFErrorGetCode(error) };
    // SAFETY: `error` is a valid CFError; the description is owned by the caller.
    let description = Owned(unsafe { CFErrorCopyDescription(error) });
    let kind = if string(domain) == COCOA_ERROR_DOMAIN {
        cocoa_error_kind(code)
    } else {
        io::ErrorKind::Other
    };
    io::Error::new(kind, string(description.0))
}

/// The kind of a file error of the Cocoa domain (`NSFileNoSuchFileError` and the like).
fn cocoa_error_kind(code: CfIndex) -> io::ErrorKind {
    match code {
        4 | 260 => io::ErrorKind::NotFound,
        257 | 513 => io::ErrorKind::PermissionDenied,
        640 => io::ErrorKind::StorageFull,
        _ => io::ErrorKind::Other,
    }
}

/// The CFString `value` as UTF-8, or an empty string for null.
fn string(value: Object) -> String {
    if value.is_null() {
        return String::new();
    }
    // SAFETY: `value` is a valid CFString.
    let length = unsafe { CFStringGetLength(value) };
    let size = CFStringGetMaximumSizeForEncoding(length, UTF8).saturating_add(1);
    let mut buffer = vec![0_u8; usize::try_from(size).unwrap_or(0)];
    // SAFETY: `value` is a valid CFString and `buffer` is writable for `size` bytes.
    let copied = unsafe { CFStringGetCString(value, buffer.as_mut_ptr().cast(), size, UTF8) };
    let text = CStr::from_bytes_until_nul(&buffer)
        .ok()
        .filter(|_| copied != 0);
    text.map(|text| text.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// A CoreFoundation object the caller owns, released when dropped.
struct Owned(Object);

impl Owned {
    /// A new file URL for `path`.
    fn file_url(path: &Path) -> io::Result<Self> {
        let bytes = path.as_os_str().as_bytes();
        let length = CfIndex::try_from(bytes.len()).map_err(io::Error::other)?;
        // SAFETY: `bytes` is readable for `length` bytes; the URL copies them.
        let url = unsafe {
            CFURLCreateFromFileSystemRepresentation(ptr::null(), bytes.as_ptr(), length, 0)
        };
        if url.is_null() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "the path is no file URL",
            ));
        }
        Ok(Owned(url))
    }
}

impl Drop for Owned {
    fn drop(&mut self) {
        if !self.0.is_null() {
            // SAFETY: the object is owned here and released exactly once.
            unsafe { CFRelease(self.0) };
        }
    }
}

/// An autorelease pool that drains the objects the calls return when dropped.
struct AutoreleasePool(*mut c_void);

impl AutoreleasePool {
    fn push() -> Self {
        AutoreleasePool(objc_autoreleasePoolPush())
    }
}

impl Drop for AutoreleasePool {
    fn drop(&mut self) {
        // SAFETY: the pool was pushed on this thread and is popped exactly once,
        // after every object autoreleased into it was last used.
        unsafe { objc_autoreleasePoolPop(self.0) };
    }
}
