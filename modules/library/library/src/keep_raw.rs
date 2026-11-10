// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Marking an asset Keep RAW in the sidecar of every member (LIBRARY-002.7).

use std::collections::BTreeSet;
use std::path::PathBuf;

use metadata::xmp::{SidecarError, SidecarErrorKind, SidecarFiles, SidecarNaming};

use crate::grouping::Asset;

/// Marks `asset` Keep RAW, or unmarks it, in the sidecar of every member.
///
/// A member without a sidecar of its own or a shared one gets one named by `naming`.
/// Every sidecar is checked before the first is written, so a refused change leaves
/// all of them as they were (LIBRARY-002.6).
///
/// # Errors
///
/// A [`SidecarError`] naming the sidecar that cannot be read, holds a
/// `framehive:KeepRaw` of another form, or cannot be written. A write error can leave
/// the sidecars before it changed.
pub fn set_keep_raw(
    sidecars: SidecarFiles<'_>,
    naming: SidecarNaming,
    asset: &Asset,
    marked: bool,
) -> Result<(), SidecarError> {
    let targets = marker_sidecars(asset, naming);
    for path in &targets {
        let mut packet = sidecars.read(path)?.packet;
        packet.set_keep_raw(marked).map_err(|edit| SidecarError {
            path: path.clone(),
            kind: SidecarErrorKind::Edit(edit),
            part_left: None,
        })?;
    }
    for path in &targets {
        sidecars.merge(path, |packet| packet.set_keep_raw(marked))?;
    }
    Ok(())
}

/// The sidecars that hold the asset's marker: those it has, and one by `naming` for
/// each member that has neither its own nor a shared one.
fn marker_sidecars(asset: &Asset, naming: SidecarNaming) -> BTreeSet<PathBuf> {
    let mut targets: BTreeSet<PathBuf> = asset
        .xmp_sidecars()
        .map(|sidecar| sidecar.path.clone())
        .collect();
    let shared = asset.shared_sidecar().is_some();
    for media in asset.media() {
        if !shared && asset.own_sidecars(media).next().is_none() {
            targets.insert(naming.sidecar_path(&media.path));
        }
    }
    targets
}
