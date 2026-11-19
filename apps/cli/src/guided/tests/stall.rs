// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The stall limit of a card in the guided import (IMPORT-022).

use std::thread;
use std::time::Duration;

use files::testing::{Fault, Hang};
use import::DEFAULT_SOURCE_STALL_LIMIT;

use super::support::{Harness, OCTOBER_17, PICTURES, PRESET_1, mount};

/// spec: IMPORT-022.5
#[test]
fn a_card_whose_listing_hangs_fails_its_scan() {
    let harness = Harness::new();
    harness.remember(Some(PICTURES), Some(PRESET_1), &[]);
    harness.add_file(
        "EOS_DIGITAL",
        "DCIM/100_FUJI/DSCF0001.RAF",
        b"raf",
        OCTOBER_17,
    );
    harness.insert("EOS_DIGITAL");
    let hang = Hang::new();
    harness
        .memory
        .inject_fault(&mount("EOS_DIGITAL"), Fault::Hangs(hang.clone()));
    let helper = {
        let (hang, timer) = (hang.clone(), harness.timer.clone());
        thread::spawn(move || {
            hang.wait_until_begun();
            while timer.waits() == 0 {
                thread::yield_now();
            }
            timer.advance(DEFAULT_SOURCE_STALL_LIMIT + Duration::from_secs(1));
        })
    };

    let transcript = harness.run(vec![]);

    hang.release();
    helper.join().unwrap();
    assert!(
        transcript.output.contains(
            "Could not read EOS_DIGITAL: /run/media/ada/EOS_DIGITAL: \
             the source did not answer for 30 seconds\n"
        ),
        "{}",
        transcript.output
    );
}
