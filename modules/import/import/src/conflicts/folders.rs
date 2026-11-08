// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The folders of a target path at the destination, read once each, and the folders
//! another destination of the run needs (IMPORT-003, IMPORT-017).

use std::collections::HashMap;
use std::io;

use files::EntryKind;

use super::target_names::{caseless_key, join_target};
use super::{ConflictError, Existing, TargetResolver, cannot_read};

/// The folders at the destination that match a target folder.
pub(super) enum Folders {
    /// The matching folders, spelled as found; none when the folder is missing.
    Found(Vec<String>),
    /// A folder of the path exists only as this file, relative to the destination.
    FileInTheWay(String),
}

impl TargetResolver<'_> {
    /// Counts `folder`, relative to the destination with `/` between segments, and
    /// each folder on its path as taken: another destination of the run lies there.
    #[must_use]
    pub(crate) fn with_needed_folder(mut self, folder: &str) -> Self {
        let mut path = String::new();
        for segment in folder.split('/').filter(|segment| !segment.is_empty()) {
            path = join_target(&path, segment);
            self.needed_folders.insert(caseless_key(&path));
        }
        self
    }

    /// The folders at the destination whose paths equal `folder`, ignoring case and
    /// normalization form in every segment, spelled as found there. A destination
    /// that tells such names apart may hold several of them. A segment found only as
    /// a file stops the search: that file is in the way (IMPORT-017).
    pub(super) fn matching_folders(&mut self, folder: &str) -> Result<Folders, ConflictError> {
        let mut found = vec![String::new()];
        for segment in folder.split('/').filter(|segment| !segment.is_empty()) {
            let key = caseless_key(segment);
            let mut next = Vec::new();
            let mut files = Vec::new();
            for parent in &found {
                let Some(matches) = self.listing(parent)?.get(&key) else {
                    continue;
                };
                // A symbolic link may lead to a folder, so only a file is no folder.
                for entry in matches {
                    if entry.kind == EntryKind::File {
                        files.push(entry.target.clone());
                    } else {
                        next.push(entry.target.clone());
                    }
                }
            }
            if next.is_empty() {
                // Listings come in no fixed order; the smallest path keeps it stable.
                return Ok(files
                    .into_iter()
                    .min()
                    .map_or(Folders::Found(Vec::new()), Folders::FileInTheWay));
            }
            found = next;
        }
        Ok(Folders::Found(found))
    }

    /// The listing of the folder `folder` at the destination, read on first use.
    pub(super) fn listing(
        &mut self,
        folder: &str,
    ) -> Result<&HashMap<String, Vec<Existing>>, ConflictError> {
        if !self.folders.contains_key(folder) {
            let listing = self.list_folder(folder)?;
            self.folders.insert(folder.to_owned(), listing);
        }
        Ok(&self.folders[folder])
    }

    fn list_folder(&self, folder: &str) -> Result<HashMap<String, Vec<Existing>>, ConflictError> {
        let path = self.destination.join(folder);
        let entries = match self.file_system.list_folder(&path) {
            Ok(entries) => entries,
            Err(error) if error.kind() == io::ErrorKind::NotFound => Vec::new(),
            Err(error) => return Err(cannot_read(&path)(error)),
        };
        let mut listing: HashMap<String, Vec<Existing>> = HashMap::new();
        for entry in entries {
            let name = entry.name.to_string_lossy().into_owned();
            listing
                .entry(caseless_key(&name))
                .or_default()
                .push(Existing {
                    target: join_target(folder, &name),
                    kind: entry.kind,
                });
        }
        Ok(listing)
    }
}
