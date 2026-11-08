// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The source `.xmp` sidecars the scan attaches to the file each one names
//! (IMPORT-020).

use std::collections::HashMap;
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};

use domain::AssetFileRole;
use metadata::xmp::SidecarNaming;

use super::{FoundFile, LeftOut, LeftOutReason};

/// A source `.xmp` sidecar that comes with the file it names (IMPORT-020).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceSidecar {
    /// The path relative to the source, e.g. `DCIM/100_FUJI/DSCF0001.RAF.xmp`.
    pub path: PathBuf,
    /// The naming form, which the sidecar keeps at the target.
    pub naming: SidecarNaming,
    /// The size in bytes.
    pub size: u64,
}

/// A `.xmp` file the scan found, not yet attached.
pub(super) struct Candidate {
    pub(super) path: PathBuf,
    pub(super) size: u64,
}

/// Whether `path` has the extension `xmp`, in any case.
pub(super) fn is_xmp(path: &Path) -> bool {
    path.extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("xmp"))
}

/// Attaches each candidate, in the given order, to the file it names, and returns the
/// candidates that stay left out.
pub(super) fn attach(files: &mut [FoundFile], candidates: Vec<Candidate>) -> Vec<LeftOut> {
    let index = index_by_base_name(files);
    let mut left_out = Vec::new();
    for candidate in candidates {
        match owner(files, &index, &candidate.path) {
            Some((owner, naming)) => {
                let sidecars = &mut files[owner].sidecars;
                if sidecars.iter().any(|sidecar| sidecar.naming == naming) {
                    left_out.push(left(candidate.path, LeftOutReason::SidecarNeedsGrouping));
                } else {
                    sidecars.push(SourceSidecar {
                        path: candidate.path,
                        naming,
                        size: candidate.size,
                    });
                }
            }
            None => left_out.push(left(candidate.path, LeftOutReason::SidecarWithoutItsFile)),
        }
    }
    left_out
}

type BaseNameIndex = HashMap<(PathBuf, OsString), Vec<usize>>;

/// The positions of the files by folder and file name without extension.
fn index_by_base_name(files: &[FoundFile]) -> BaseNameIndex {
    let mut index: BaseNameIndex = HashMap::new();
    for (position, file) in files.iter().enumerate() {
        if let Some(key) = folder_and_stem(&file.path) {
            index.entry(key).or_default().push(position);
        }
    }
    index
}

fn folder_and_stem(path: &Path) -> Option<(PathBuf, OsString)> {
    let folder = path.parent().unwrap_or(Path::new("")).to_path_buf();
    Some((folder, path.file_stem()?.to_os_string()))
}

/// The file `sidecar` names and the naming form: `X.RAF.xmp` names `X.RAF`, `X.xmp`
/// names the file with base name `X`, the RAW when several share it.
fn owner(
    files: &[FoundFile],
    index: &BaseNameIndex,
    sidecar: &Path,
) -> Option<(usize, SidecarNaming)> {
    let (folder, named) = folder_and_stem(sidecar)?;
    let named = Path::new(&named);
    if let (Some(stem), Some(extension)) = (named.file_stem(), named.extension()) {
        let with_extension = index
            .get(&(folder.clone(), stem.to_os_string()))
            .and_then(|positions| file_with_extension(files, positions, extension));
        if let Some(position) = with_extension {
            return Some((position, SidecarNaming::OriginalWithExtension));
        }
    }
    let positions = index.get(&(folder, named.as_os_str().to_os_string()))?;
    let raw = positions
        .iter()
        .find(|position| files[**position].role == AssetFileRole::Raw);
    let position = raw.or(positions.first())?;
    Some((*position, SidecarNaming::OriginalWithoutExtension))
}

fn file_with_extension(files: &[FoundFile], positions: &[usize], wanted: &OsStr) -> Option<usize> {
    positions.iter().copied().find(|position| {
        files[*position]
            .path
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case(wanted))
    })
}

fn left(path: PathBuf, reason: LeftOutReason) -> LeftOut {
    LeftOut { path, reason }
}
