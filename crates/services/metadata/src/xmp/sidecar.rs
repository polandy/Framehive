// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The names of sidecars and finding them next to their file (CORE-010, sidecar
//! names; ADR 0040 §1).

use std::ffi::OsStr;
use std::io;
use std::path::{Path, PathBuf};

use files::{EntryKind, ReadOnlyFileSystem};

const SIDECAR_EXTENSION: &str = ".xmp";

/// How a sidecar is named after its file (ADR 0040 §1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum SidecarNaming {
    /// The file's full name plus `.xmp`, such as `DSCF0042.RAF.xmp`.
    #[default]
    OriginalWithExtension,
    /// The file's name with its extension replaced by `xmp`, such as `DSCF0042.xmp`.
    OriginalWithoutExtension,
}

impl SidecarNaming {
    /// The path of the sidecar of `file`, in the same folder.
    ///
    /// ```
    /// use std::path::Path;
    ///
    /// use metadata::xmp::SidecarNaming;
    ///
    /// let file = Path::new("/photos/DSCF0042.RAF");
    /// assert_eq!(
    ///     SidecarNaming::OriginalWithoutExtension.sidecar_path(file),
    ///     Path::new("/photos/DSCF0042.xmp")
    /// );
    /// ```
    #[must_use]
    pub fn sidecar_path(self, file: &Path) -> PathBuf {
        match self {
            SidecarNaming::OriginalWithExtension => {
                let mut name = file.as_os_str().to_owned();
                name.push(SIDECAR_EXTENSION);
                PathBuf::from(name)
            }
            SidecarNaming::OriginalWithoutExtension => file.with_extension("xmp"),
        }
    }
}

/// The sidecars of `file` in its folder, in either naming and with the extension in
/// any case: the one named after the full name first.
///
/// # Errors
///
/// Returns an error if the folder of `file` cannot be listed.
pub fn find_sidecars(files: &dyn ReadOnlyFileSystem, file: &Path) -> io::Result<Vec<PathBuf>> {
    let (Some(folder), Some(name)) = (file.parent(), file.file_name()) else {
        return Ok(Vec::new());
    };
    let stem = Path::new(name)
        .extension()
        .and(Path::new(name).file_stem())
        .map(OsStr::as_encoded_bytes);
    let mut found = Vec::new();
    for entry in files.list_folder(folder)? {
        let Some(base) = sidecar_base(&entry.name).filter(|_| entry.kind == EntryKind::File) else {
            continue;
        };
        if base == name.as_encoded_bytes() {
            found.push((0, entry.name));
        } else if Some(base) == stem {
            found.push((1, entry.name));
        }
    }
    found.sort();
    Ok(found
        .into_iter()
        .map(|(_, sidecar)| folder.join(sidecar))
        .collect())
}

/// The name of a sidecar without its extension, which matches in any case.
fn sidecar_base(name: &OsStr) -> Option<&[u8]> {
    let bytes = name.as_encoded_bytes();
    let (base, extension) = bytes.split_at(bytes.len().checked_sub(SIDECAR_EXTENSION.len())?);
    extension
        .eq_ignore_ascii_case(SIDECAR_EXTENSION.as_bytes())
        .then_some(base)
}
