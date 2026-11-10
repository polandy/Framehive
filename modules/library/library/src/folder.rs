// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The assets of a folder with their Keep RAW marker and rejected state, read back
//! from their sidecars (LIBRARY-001, LIBRARY-002).

use std::io;
use std::path::{Path, PathBuf};

use domain::{AssetFileRole, Timestamp};
use files::EntryKind;
use metadata::xmp::{SidecarError, SidecarFiles, XmpPacket};
use thiserror::Error;

use crate::grouping::{AmbiguousBaseName, Asset, AssetFile, FolderFile, group_files};

/// An asset with the state Remove RAWs needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LibraryAsset {
    /// The asset and its files.
    pub asset: Asset,
    /// Whether the asset is marked Keep RAW.
    pub keep_raw: bool,
    /// Whether the asset's pick state is rejected.
    pub rejected: bool,
}

/// The assets of one folder.
#[derive(Debug, Default)]
pub struct LibraryFolder {
    /// The assets whose sidecars could be read.
    pub assets: Vec<LibraryAsset>,
    /// The base names whose files did not group (LIBRARY-001.4).
    pub ambiguous: Vec<AmbiguousBaseName>,
    /// The sidecars that could not be read; their assets are left out, so no command
    /// acts on an asset whose state is unknown.
    pub unreadable: Vec<SidecarError>,
}

/// A folder or a file in it could not be inspected.
#[derive(Debug, Error)]
#[error("cannot read {}: {source}", path.display())]
pub struct ReadFolderError {
    /// The folder or file.
    pub path: PathBuf,
    /// Why.
    #[source]
    pub source: io::Error,
}

/// Reads the assets of the files directly in `folder`.
///
/// When the sidecars of an asset disagree, the most recently modified one holds its
/// state, and on a tie the RAW's own sidecar (LIBRARY-001).
///
/// # Errors
///
/// A [`ReadFolderError`] when `folder` cannot be listed or a file's size or a
/// sidecar's modification time cannot be read.
pub fn read_folder(
    sidecars: SidecarFiles<'_>,
    folder: &Path,
) -> Result<LibraryFolder, ReadFolderError> {
    let grouping = group_files(folder, &folder_files(sidecars, folder)?);
    let mut library_folder = LibraryFolder {
        ambiguous: grouping.ambiguous,
        ..LibraryFolder::default()
    };
    for asset in grouping.assets {
        match leading_packet(sidecars, &asset)? {
            Ok(packet) => library_folder.assets.push(LibraryAsset {
                keep_raw: packet.keep_raw(),
                rejected: packet.is_rejected(),
                asset,
            }),
            Err(error) => library_folder.unreadable.push(error),
        }
    }
    Ok(library_folder)
}

fn folder_files(
    sidecars: SidecarFiles<'_>,
    folder: &Path,
) -> Result<Vec<FolderFile>, ReadFolderError> {
    let entries = sidecars
        .files
        .list_folder(folder)
        .map_err(|source| read_error(folder, source))?;
    let mut files = Vec::new();
    for entry in entries
        .into_iter()
        .filter(|entry| entry.kind == EntryKind::File)
    {
        // A name that is not UTF-8 cannot be compared by base name.
        let Ok(name) = entry.name.into_string() else {
            continue;
        };
        let path = folder.join(&name);
        let size_bytes = sidecars
            .files
            .file_size(&path)
            .map_err(|source| read_error(&path, source))?;
        files.push(FolderFile { name, size_bytes });
    }
    Ok(files)
}

/// The packet that holds the asset's state; an empty one when it has no sidecar.
fn leading_packet(
    sidecars: SidecarFiles<'_>,
    asset: &Asset,
) -> Result<Result<XmpPacket, SidecarError>, ReadFolderError> {
    let Some(leading) = leading_sidecar(sidecars, asset)? else {
        return Ok(Ok(XmpPacket::new()));
    };
    let mut packet = Ok(XmpPacket::new());
    for sidecar in asset.xmp_sidecars() {
        // Every sidecar is read, so an unreadable one keeps the asset out.
        let read = sidecars.read(&sidecar.path).map(|read| read.packet);
        match read {
            Err(error) => return Ok(Err(error)),
            Ok(read) if sidecar.path == leading.path => packet = Ok(read),
            Ok(_) => {}
        }
    }
    Ok(packet)
}

fn leading_sidecar<'a>(
    sidecars: SidecarFiles<'_>,
    asset: &'a Asset,
) -> Result<Option<&'a AssetFile>, ReadFolderError> {
    let raw_own: Vec<&AssetFile> = asset
        .file(AssetFileRole::Raw)
        .map(|raw| asset.own_sidecars(raw).collect())
        .unwrap_or_default();
    let others = asset
        .xmp_sidecars()
        .filter(|sidecar| !raw_own.contains(sidecar));
    let mut leading: Option<(&AssetFile, Timestamp)> = None;
    for sidecar in raw_own.iter().copied().chain(others) {
        let modified = sidecars
            .files
            .modification_time(&sidecar.path)
            .map_err(|source| read_error(&sidecar.path, source))?;
        if leading.is_none_or(|(_, newest)| modified > newest) {
            leading = Some((sidecar, modified));
        }
    }
    Ok(leading.map(|(sidecar, _)| sidecar))
}

fn read_error(path: &Path, source: io::Error) -> ReadFolderError {
    ReadFolderError {
        path: path.to_path_buf(),
        source,
    }
}
