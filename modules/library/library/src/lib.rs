// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The `library` module of Framehive: business logic, use cases and ViewModels
//! (architecture §4.1). Its views live in `library-ui`. See `README.md`.

#![forbid(unsafe_code)]

mod folder;
mod grouping;
mod keep_raw;
mod remove_raws;
mod view_model;

use kernel::{ModuleId, ModuleIdError};

pub use folder::{LibraryAsset, LibraryFolder, ReadFolderError, read_folder};
pub use grouping::{AmbiguousBaseName, Asset, AssetFile, FolderFile, Grouping, group_files};
pub use keep_raw::set_keep_raw;
pub use remove_raws::{
    NotMoved, RemoveRawsPlan, RemoveRawsReport, RemoveRawsTotals, SkippedAssets,
};
pub use view_model::ExampleViewModel;

/// Stable, unique id of this module (architecture §4.2).
pub const MODULE_ID: &str = "library";

/// Returns [`MODULE_ID`] as a validated [`ModuleId`].
///
/// # Errors
///
/// Returns a [`ModuleIdError`] if [`MODULE_ID`] is not a valid module id; the
/// conformance test rules that out.
pub fn module_id() -> Result<ModuleId, ModuleIdError> {
    ModuleId::new(MODULE_ID)
}
