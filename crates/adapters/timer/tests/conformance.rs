// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Port conformance test: every adapter must pass the conformance suite of the port
//! it implements (CLAUDE.md, "Adding things").

use adapter_timer::StdTimer;
use files::testing::{TimerUnderTest, check_timer};

#[test]
fn conforms_to_the_port() {
    let failures = check_timer(&TimerUnderTest {
        timer: &StdTimer::new(),
    });

    assert_eq!(failures, Vec::<String>::new());
}
