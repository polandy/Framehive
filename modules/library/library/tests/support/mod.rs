// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! A library folder in memory, with its sidecars and a trash.

#![allow(clippy::unwrap_used, reason = "tests may panic on bad fixtures")]
#![allow(dead_code, reason = "each test file uses part of the fixture")]

use std::path::{Path, PathBuf};

use domain::testing::ManualClock;
use domain::{Timestamp, UtcOffset};
use files::testing::{FnvHasher, MemoryFileSystem, MemoryTrash};
use library::{LibraryAsset, LibraryFolder, read_folder};
use metadata::testing::RecordedXml;
use metadata::xmp::namespaces::XMP_BASIC;
use metadata::xmp::{SidecarFiles, XmpName, XmpPacket, serialize_xmp};

/// The folder the fixture's files are in.
pub(crate) const FOLDER: &str = "/photos";

/// The ports of the library, with the files in memory.
pub(crate) struct Library {
    pub(crate) files: MemoryFileSystem,
    pub(crate) trash: MemoryTrash,
    hasher: FnvHasher,
    xml: RecordedXml,
    clock: ManualClock,
}

impl Library {
    /// An empty folder [`FOLDER`] and an empty trash.
    pub(crate) fn new() -> Self {
        let files = MemoryFileSystem::new();
        files.add_folder(Path::new(FOLDER));
        Library {
            trash: MemoryTrash::new(files.clone(), "/trash"),
            files,
            hasher: FnvHasher,
            xml: RecordedXml::default(),
            clock: ManualClock::new(
                Timestamp::from_unix_seconds(1_794_142_992),
                UtcOffset::from_seconds(3_600).unwrap(),
            ),
        }
    }

    /// Adds the media file `name` with `size_bytes` bytes.
    pub(crate) fn add_media(&self, name: &str, size_bytes: usize) {
        self.files
            .add_file(&path(name), &vec![0; size_bytes], Timestamp::UNIX_EPOCH);
    }

    /// Adds the sidecar `name` holding `packet`, modified at `modified_seconds`.
    pub(crate) fn add_sidecar(&self, name: &str, packet: &XmpPacket, modified_seconds: i64) {
        let content = serialize_xmp(packet, &self.clock, &self.xml).unwrap();
        self.files.add_file(
            &path(name),
            content.as_bytes(),
            Timestamp::from_unix_seconds(modified_seconds),
        );
    }

    pub(crate) fn sidecars(&self) -> SidecarFiles<'_> {
        SidecarFiles {
            files: &self.files,
            hasher: &self.hasher,
            xml: &self.xml,
            clock: &self.clock,
        }
    }

    /// Reads the folder, as an index rebuild does.
    pub(crate) fn read(&self) -> LibraryFolder {
        read_folder(self.sidecars(), Path::new(FOLDER)).unwrap()
    }

    /// The asset of the folder with `base_name`.
    pub(crate) fn asset(&self, base_name: &str) -> LibraryAsset {
        self.read()
            .assets
            .into_iter()
            .find(|asset| asset.asset.base_name() == base_name)
            .unwrap()
    }

    /// The packet of the sidecar `name`.
    pub(crate) fn packet(&self, name: &str) -> XmpPacket {
        self.sidecars().read(&path(name)).unwrap().packet
    }

    /// Whether `name` is still in the folder.
    pub(crate) fn exists(&self, name: &str) -> bool {
        self.files.content(&path(name)).is_some()
    }
}

/// The path of `name` in [`FOLDER`].
pub(crate) fn path(name: &str) -> PathBuf {
    Path::new(FOLDER).join(name)
}

/// The paths of `names` in [`FOLDER`].
pub(crate) fn paths(names: &[&str]) -> Vec<PathBuf> {
    names.iter().map(|name| path(name)).collect()
}

/// A packet with `xmp:Rating` `rating`.
pub(crate) fn rated(rating: &str) -> XmpPacket {
    let mut packet = XmpPacket::new();
    packet.set_text(&rating_name(), rating).unwrap();
    packet
}

/// A packet marked Keep RAW.
pub(crate) fn marked() -> XmpPacket {
    let mut packet = XmpPacket::new();
    packet.set_keep_raw(true).unwrap();
    packet
}

pub(crate) fn rating_name() -> XmpName {
    XmpName::new(XMP_BASIC, "Rating")
}
