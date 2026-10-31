// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The cases of the [`FileSystem`](crate::FileSystem) conformance suite that list
//! folders and read file sizes (ADR 0020 §4, amendment).

use std::io;
use std::path::Path;

use super::{Case, CaseResult, ensure, expect_error_kind, failed};
use crate::{EntryKind, FolderEntry};

impl Case<'_> {
    /// Compares the listing of `folder`, sorted by name, with `expected`.
    pub(super) fn expect_listing(&self, folder: &Path, expected: &[FolderEntry]) -> CaseResult {
        let mut listing = self
            .fs()
            .list_folder(folder)
            .map_err(failed("list_folder"))?;
        listing.sort_by(|a, b| a.name.cmp(&b.name));
        ensure(
            listing == expected,
            format!(
                "{} lists {listing:?}, expected {expected:?}",
                folder.display()
            ),
        )
    }
}

pub(super) fn list_folder_names_entries(case: &Case<'_>) -> CaseResult {
    case.write(&case.path("DSCF0001.RAF"), b"raw")?;
    case.fs()
        .create_folder(&case.path("DCIM"))
        .map_err(failed("create_folder"))?;
    let expected = vec![
        FolderEntry {
            name: "DCIM".into(),
            kind: EntryKind::Folder,
        },
        FolderEntry {
            name: "DSCF0001.RAF".into(),
            kind: EntryKind::File,
        },
    ];
    case.expect_listing(&case.folder, &expected)?;
    case.expect_listing(&case.path("DCIM"), &[])?;
    expect_error_kind(
        case.fs().list_folder(&case.path("missing")),
        io::ErrorKind::NotFound,
        "listing a missing folder",
    )?;
    ensure(
        case.fs().list_folder(&case.path("DSCF0001.RAF")).is_err(),
        "listing a file succeeded".to_owned(),
    )
}

/// A link to its own folder would make a scan loop if it were followed (IMPORT-005.5).
pub(super) fn list_folder_does_not_follow_links(case: &Case<'_>) -> CaseResult {
    let Some(make_symbolic_link) = case.under_test.make_symbolic_link else {
        return Ok(());
    };
    let photo = case.path("photo.jpg");
    case.write(&photo, b"jpeg")?;
    let loop_link = case.path("all");
    let file_link = case.path("link.jpg");
    make_symbolic_link(&loop_link, &case.folder).map_err(failed("make_symbolic_link"))?;
    make_symbolic_link(&file_link, &photo).map_err(failed("make_symbolic_link"))?;
    let expected = vec![
        FolderEntry {
            name: "all".into(),
            kind: EntryKind::SymbolicLink,
        },
        FolderEntry {
            name: "link.jpg".into(),
            kind: EntryKind::SymbolicLink,
        },
        FolderEntry {
            name: "photo.jpg".into(),
            kind: EntryKind::File,
        },
    ];
    case.expect_listing(&case.folder, &expected)?;
    case.expect_entry(&loop_link, Some(EntryKind::SymbolicLink))?;
    case.expect_entry(&file_link, Some(EntryKind::SymbolicLink))
}

pub(super) fn file_size(case: &Case<'_>) -> CaseResult {
    let path = case.path("photo.raf");
    case.write(&path, &[7; 1_024])?;
    let size = case.fs().file_size(&path).map_err(failed("file_size"))?;
    ensure(size == 1_024, format!("file_size is {size}, expected 1024"))?;
    expect_error_kind(
        case.fs().file_size(&case.path("missing")),
        io::ErrorKind::NotFound,
        "file_size of a missing file",
    )?;
    ensure(
        case.fs().file_size(&case.folder).is_err(),
        "file_size of a folder succeeded".to_owned(),
    )
}
