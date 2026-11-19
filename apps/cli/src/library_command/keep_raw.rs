// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! `framehive library keep-raw` (CLI-018): marks the assets of the named files Keep
//! RAW, or unmarks them.

use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};

use library::{Asset, LibraryFolder, set_keep_raw};
use metadata::xmp::{SidecarFiles, SidecarNaming};

use super::{asset_name, read_folder_message, read_library_folder, write_line};
use crate::{FILES_FAILED, NOT_STARTED, write_error};

/// What `framehive library keep-raw` changes (CLI-018).
#[derive(Debug, Clone, Copy)]
pub(crate) struct KeepRawChange {
    /// Mark the assets, or unmark them (`--off`).
    pub(crate) marked: bool,
    /// `--dry-run`: print the lines, write no sidecar.
    pub(crate) dry_run: bool,
}

/// Sets the Keep RAW marker of the assets of `files`, which are absolute, as `change`
/// says, and returns the exit code of CLI-018.
pub(crate) fn keep_raw(
    sidecars: SidecarFiles<'_>,
    naming: SidecarNaming,
    files: &[PathBuf],
    change: KeepRawChange,
    output: &mut dyn Write,
    errors: &mut dyn Write,
) -> u8 {
    let assets = match named_assets(sidecars, files, errors) {
        Ok(assets) => assets,
        Err(exit_code) => return exit_code,
    };
    let state = match (change.dry_run, change.marked) {
        (false, true) => "on",
        (false, false) => "off",
        (true, true) => "would be on",
        (true, false) => "would be off",
    };
    let mut exit_code = 0;
    for asset in &assets {
        let set = if change.dry_run {
            Ok(())
        } else {
            set_keep_raw(sidecars, naming, asset, change.marked)
        };
        match set {
            Ok(()) => write_line(output, &format!("Keep RAW {state}: {}", asset_name(asset))),
            Err(error) => {
                let name = asset_name(asset);
                write_error(errors, &format!("cannot set Keep RAW of {name}: {error}"));
                exit_code = FILES_FAILED;
            }
        }
    }
    exit_code
}

/// The assets that `files` name, each once, in the order of `files`; or the exit code
/// when a folder cannot be read or a file is not part of an asset.
fn named_assets(
    sidecars: SidecarFiles<'_>,
    files: &[PathBuf],
    errors: &mut dyn Write,
) -> Result<Vec<Asset>, u8> {
    let mut folders: BTreeMap<PathBuf, LibraryFolder> = BTreeMap::new();
    let mut assets: Vec<Asset> = Vec::new();
    let mut all_found = true;
    for file in files {
        let folder = file.parent().unwrap_or(Path::new("/"));
        if !folders.contains_key(folder) {
            let library_folder =
                read_library_folder(sidecars, folder, errors).map_err(|error| {
                    write_error(errors, &read_folder_message(&error));
                    NOT_STARTED
                })?;
            folders.insert(folder.to_path_buf(), library_folder);
        }
        match folders.get(folder).and_then(|found| asset_of(found, file)) {
            Some(asset) if assets.iter().any(|known| known.id() == asset.id()) => {}
            Some(asset) => assets.push(asset.clone()),
            None => {
                let message = format!("{} is not a photo or video of the library", file.display());
                write_error(errors, &message);
                all_found = false;
            }
        }
    }
    if all_found {
        Ok(assets)
    } else {
        Err(NOT_STARTED)
    }
}

/// The asset of `folder` that `file` is part of.
fn asset_of<'a>(folder: &'a LibraryFolder, file: &Path) -> Option<&'a Asset> {
    folder
        .assets
        .iter()
        .map(|library_asset| &library_asset.asset)
        .find(|asset| asset.files().iter().any(|member| member.path == file))
}
