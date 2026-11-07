// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The free space of the destinations at the plan summary (CLI-011).

use std::path::Path;
use std::sync::Arc;

use files::testing::MemoryFreeSpace;

use super::support::{Harness, OCTOBER_17, PICTURES, PRESET_1, Step};

/// The folder of the other destination of CLI-011.6, on its own file system.
const ARCHIVE: &str = "/home/ada/Archive";

const SHORT: &str = "Not enough space at /home/ada/Pictures: needs 2.5 MB, 1.0 MB free; \
                     type d or b to change a folder.\n";

/// A harness with the card `EOS_DIGITAL` with `DSCF0001.RAF` of 2.5 MB, the
/// destination `Pictures` on a file system with 1.0 MB free, and `Archive` on one with
/// 10.0 MB free.
fn with_short_destination() -> Harness {
    let mut harness = Harness::new();
    harness.remember(Some(PICTURES), Some(PRESET_1), &[]);
    harness.insert("EOS_DIGITAL");
    harness.add_file(
        "EOS_DIGITAL",
        "DCIM/100_FUJI/DSCF0001.RAF",
        b"first",
        OCTOBER_17,
    );
    harness.pretend_size("EOS_DIGITAL", "DCIM/100_FUJI/DSCF0001.RAF", 2_500_000);
    harness.free_space = Arc::new(
        MemoryFreeSpace::unlimited()
            .with_file_system(PICTURES, 1_000_000)
            .with_file_system(ARCHIVE, 10_000_000),
    );
    harness
}

/// spec: CLI-011.5
#[test]
fn a_short_destination_keeps_enter_from_starting() {
    let harness = with_short_destination();

    let transcript = harness.run(vec![Step::Line("")]);

    assert!(
        transcript.output.contains(&format!(
            "\x20 named     {PRESET_1}\n{SHORT}[Enter] import  [l] list  [t] template  [d] destination  [b] backup  [q] quit: \n{SHORT}"
        )),
        "{}",
        transcript.output
    );
    assert!(
        !harness
            .memory
            .paths()
            .iter()
            .any(|path| path.starts_with(PICTURES))
    );
}

/// spec: CLI-011.6
#[test]
fn changing_the_destination_checks_again() {
    let harness = with_short_destination();

    let transcript = harness.run(vec![Step::Line("d"), Step::Line(ARCHIVE), Step::Line("")]);

    let output = &transcript.output;
    let changed = output.find(&format!("]: {ARCHIVE}\n")).unwrap();
    assert!(!output[changed..].contains("Not enough space"), "{output}");
    assert!(
        harness
            .memory
            .paths()
            .contains(&Path::new(ARCHIVE).join("2026/2026-10-17/DSCF0001.RAF")),
        "{output}"
    );
}

/// spec: CLI-011.7
#[test]
fn unknown_free_space_warns_and_enter_starts() {
    let mut harness = with_short_destination();
    harness.free_space = Arc::new(
        MemoryFreeSpace::unlimited().with_unreadable(PICTURES, "the file system does not say"),
    );

    let transcript = harness.run(vec![Step::Line("")]);

    assert!(
        transcript.warnings.contains(
            "warning: cannot tell the free space at /home/ada/Pictures: \
             the file system does not say\n"
        ),
        "{}",
        transcript.warnings
    );
    assert!(
        harness
            .memory
            .paths()
            .contains(&Path::new(PICTURES).join("2026/2026-10-17/DSCF0001.RAF")),
        "{}",
        transcript.output
    );
}
