// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Remove RAWs: the plan with its two groups, skip counts and totals, and the run
//! that moves the RAWs to the trash asset by asset (LIBRARY-002).

use std::collections::BTreeMap;
use std::io;
use std::iter;
use std::path::{Path, PathBuf};

use domain::AssetFileRole;
use files::Trash;
use metadata::xmp::{SidecarError, SidecarFiles, SidecarNaming};

use crate::folder::{LibraryAsset, LibraryFolder, read_folder};
use crate::grouping::AssetFile;
use crate::keep_raw::set_keep_raw;

/// How many assets of the view Remove RAWs skips, and why.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SkippedAssets {
    /// Assets with a RAW and a rendered member that are rejected.
    pub rejected: usize,
    /// Assets with a rendered member and no RAW.
    pub rendered_only: usize,
    /// Assets with a RAW and no rendered member.
    pub raw_only: usize,
}

/// What Remove RAWs moves, or moved, to the trash.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RemoveRawsTotals {
    /// Assets whose RAW goes, or went, to the trash.
    pub assets: usize,
    /// Files that go, or went, to the trash: RAWs and their own sidecars.
    pub files: usize,
    /// The sum of the sizes of those files.
    pub bytes: u64,
}

impl RemoveRawsTotals {
    fn add(&mut self, file: &AssetFile) {
        self.files += 1;
        self.bytes += file.size_bytes;
    }
}

/// Remove RAWs on a view, before it runs: the summary the person confirms.
///
/// An asset with a RAW and a rendered member that is not rejected is in one of two
/// groups: *RAW will be removed*, or *RAW stays (Keep RAW)* when it is marked.
/// [`RemoveRawsPlan::toggle_keep_raw`] moves it to the other group.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoveRawsPlan {
    candidates: Vec<LibraryAsset>,
    skipped: SkippedAssets,
}

impl RemoveRawsPlan {
    /// The plan for the assets of `view`, the ones the grid shows.
    #[must_use]
    pub fn new(view: &[LibraryAsset]) -> Self {
        let mut skipped = SkippedAssets::default();
        let mut candidates = Vec::new();
        for asset in view {
            let raw = asset.asset.file(AssetFileRole::Raw).is_some();
            let rendered = asset.asset.file(AssetFileRole::Rendered).is_some();
            match (raw, rendered) {
                (true, true) if asset.rejected => skipped.rejected += 1,
                (true, true) => candidates.push(asset.clone()),
                (false, true) => skipped.rendered_only += 1,
                (true, false) => skipped.raw_only += 1,
                (false, false) => {}
            }
        }
        RemoveRawsPlan {
            candidates,
            skipped,
        }
    }

    /// The group *RAW will be removed*.
    pub fn removed(&self) -> impl Iterator<Item = &LibraryAsset> {
        self.candidates.iter().filter(|asset| !asset.keep_raw)
    }

    /// The group *RAW stays (Keep RAW)*.
    pub fn kept(&self) -> impl Iterator<Item = &LibraryAsset> {
        self.candidates.iter().filter(|asset| asset.keep_raw)
    }

    /// The assets of the view that are in neither group.
    #[must_use]
    pub fn skipped(&self) -> SkippedAssets {
        self.skipped
    }

    /// The files that go to the trash: each RAW of *RAW will be removed* and then its
    /// own sidecars.
    pub fn files(&self) -> impl Iterator<Item = &AssetFile> {
        self.removed().flat_map(files_to_trash)
    }

    /// The assets, files and bytes that go to the trash.
    #[must_use]
    pub fn totals(&self) -> RemoveRawsTotals {
        let mut totals = RemoveRawsTotals {
            assets: self.removed().count(),
            ..RemoveRawsTotals::default()
        };
        self.files().for_each(|file| totals.add(file));
        totals
    }

    /// Toggles the Keep RAW marker of the asset named `id` in its sidecars and moves
    /// it to the other group; it stays toggled when the person cancels. Returns the
    /// new marker, or `None` when `id` is in neither group.
    ///
    /// # Errors
    ///
    /// The [`SidecarError`] of [`set_keep_raw`]; the asset then stays in its group.
    pub fn toggle_keep_raw(
        &mut self,
        sidecars: SidecarFiles<'_>,
        naming: SidecarNaming,
        id: &Path,
    ) -> Result<Option<bool>, SidecarError> {
        let Some(asset) = self
            .candidates
            .iter_mut()
            .find(|asset| asset.asset.id() == id)
        else {
            return Ok(None);
        };
        let marked = !asset.keep_raw;
        set_keep_raw(sidecars, naming, &asset.asset, marked)?;
        asset.keep_raw = marked;
        Ok(Some(marked))
    }

    /// Moves the files of [`RemoveRawsPlan::files`] to `trash`, asset by asset.
    ///
    /// The folders are read again first, and an asset that changed since the plan
    /// moves no file (LIBRARY-002.18 to .20). A failure stops only its own asset: a
    /// RAW that cannot be moved keeps its sidecars, and a sidecar that cannot be moved
    /// stays where it is.
    #[must_use]
    pub fn run(&self, sidecars: SidecarFiles<'_>, trash: &dyn Trash) -> RemoveRawsReport {
        let mut report = RemoveRawsReport::default();
        let mut folders = BTreeMap::new();
        let mut current = Vec::new();
        for asset in self.removed() {
            match current_asset(sidecars, &mut folders, asset) {
                Ok(fresh) => current.push(fresh),
                Err(changed) => report.changed.push(changed),
            }
        }
        for asset in &current {
            let files = files_to_trash(asset);
            let Some((raw, own_sidecars)) = files.split_first() else {
                continue;
            };
            if !report.move_file(trash, raw) {
                continue;
            }
            report.moved.assets += 1;
            for sidecar in own_sidecars {
                report.move_file(trash, sidecar);
            }
        }
        report
    }
}

/// Why an asset of *RAW will be removed* moved no file at the run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeReason {
    /// Its RAW is no longer there.
    RawGone,
    /// Its rendered member is no longer there.
    RenderedGone,
    /// It is now marked Keep RAW.
    MarkedKeepRaw,
    /// It is now rejected.
    Rejected,
    /// Its folder or one of its sidecars can no longer be read.
    StateUnreadable,
}

/// An asset that changed since the plan, so its RAW stays.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangedAsset {
    /// The asset as `<folder>/<base>`.
    pub asset: PathBuf,
    /// What changed.
    pub reason: ChangeReason,
}

/// The asset of `planned` as its folder holds it now, or what changed.
fn current_asset(
    sidecars: SidecarFiles<'_>,
    folders: &mut BTreeMap<PathBuf, Option<LibraryFolder>>,
    planned: &LibraryAsset,
) -> Result<LibraryAsset, ChangedAsset> {
    let raw = planned
        .asset
        .file(AssetFileRole::Raw)
        .map_or_else(|| Path::new(""), |raw| raw.path.as_path());
    let folder = raw.parent().unwrap_or_else(|| Path::new(""));
    let changed = |reason| ChangedAsset {
        asset: folder.join(planned.asset.base_name()),
        reason,
    };
    let library_folder = folders
        .entry(folder.to_path_buf())
        .or_insert_with(|| read_folder(sidecars, folder).ok());
    let Some(library_folder) = library_folder else {
        return Err(changed(ChangeReason::StateUnreadable));
    };
    let fresh = library_folder.assets.iter().find(|asset| {
        asset
            .asset
            .file(AssetFileRole::Raw)
            .is_some_and(|file| file.path == raw)
    });
    let Some(fresh) = fresh else {
        // A RAW that is still there belongs to an asset left out as unreadable.
        return Err(match sidecars.files.entry(raw) {
            Ok(None) => changed(ChangeReason::RawGone),
            Ok(Some(_)) | Err(_) => changed(ChangeReason::StateUnreadable),
        });
    };
    if fresh.asset.file(AssetFileRole::Rendered).is_none() {
        Err(changed(ChangeReason::RenderedGone))
    } else if fresh.rejected {
        Err(changed(ChangeReason::Rejected))
    } else if fresh.keep_raw {
        Err(changed(ChangeReason::MarkedKeepRaw))
    } else {
        Ok(fresh.clone())
    }
}

/// A file Remove RAWs could not move to the trash, with the reason.
#[derive(Debug)]
pub struct NotMoved {
    /// The file, which is still where it was.
    pub path: PathBuf,
    /// Why the trash refused it.
    pub reason: io::Error,
}

/// What a run of Remove RAWs did.
#[derive(Debug, Default)]
pub struct RemoveRawsReport {
    /// What was moved to the trash.
    pub moved: RemoveRawsTotals,
    /// The files that stayed, in the order they were tried.
    pub not_moved: Vec<NotMoved>,
    /// The assets that changed since the plan and moved no file, in plan order.
    pub changed: Vec<ChangedAsset>,
}

impl RemoveRawsReport {
    /// Moves `file` to `trash` and counts it; returns whether it moved.
    fn move_file(&mut self, trash: &dyn Trash, file: &AssetFile) -> bool {
        match trash.move_to_trash(&file.path) {
            Ok(_) => {
                self.moved.add(file);
                true
            }
            Err(reason) => {
                self.not_moved.push(NotMoved {
                    path: file.path.clone(),
                    reason,
                });
                false
            }
        }
    }
}

/// The RAW of `asset` and then its own sidecars; a shared `<base>.xmp` stays.
fn files_to_trash(asset: &LibraryAsset) -> Vec<&AssetFile> {
    let Some(raw) = asset.asset.file(AssetFileRole::Raw) else {
        return Vec::new();
    };
    iter::once(raw)
        .chain(asset.asset.own_sidecars(raw))
        .collect()
}
