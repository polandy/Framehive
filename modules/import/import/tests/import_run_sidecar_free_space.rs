// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The free-space check counts the sidecars of the photos it writes (IMPORT-012).

#![allow(clippy::unwrap_used, reason = "tests may panic")]

mod sidecar_support;

use std::path::{Path, PathBuf};

use files::testing::MemoryFreeSpace;
use import::{RunError, SpaceShortage};
use sidecar_support::{
    DESTINATION, FLAT, Import, Machine, card_with, raw_taken_at, sidecars_below,
};

const RAW: &str = "DSCF0042.RAF";

/// spec: IMPORT-012.12
#[test]
fn a_sidecar_counts_with_its_file() {
    let photo = raw_taken_at("2026:10:17 14:05:09");
    let probe = card_with(&[(RAW, &photo)]);
    Machine::new().import(&probe, &Import::lisbon(FLAT));
    let sidecar = probe
        .content(Path::new("/library/DSCF0042.RAF.xmp"))
        .unwrap()
        .len() as u64;
    let photo_bytes = photo.len() as u64;

    let machine = Machine::with_free_space(
        MemoryFreeSpace::new().with_file_system(DESTINATION, photo_bytes + sidecar - 1),
    );
    let card = card_with(&[(RAW, &photo)]);
    let result = machine.try_import(&card, &Import::lisbon(FLAT));

    let Err(RunError::NotEnoughSpace(error)) = result else {
        panic!("not enough space expected, got {result:?}");
    };
    assert_eq!(
        error.shortages,
        [SpaceShortage {
            destinations: vec![PathBuf::from(DESTINATION)],
            bytes_to_write: photo_bytes + sidecar,
            available_bytes: photo_bytes + sidecar - 1,
        }]
    );
    assert!(sidecars_below(&card, DESTINATION).is_empty());
}
