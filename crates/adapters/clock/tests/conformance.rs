// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! `domain` has no conformance suite for the `Clock` port, and a test against the real
//! wall clock or the machine's time zone would not be deterministic (CLAUDE.md rule
//! 4). The behaviour is tested in `src/tests.rs` with a fixed time zone; here only the
//! public type is checked, without constructing it.

use adapter_clock::SystemClock;

#[test]
fn the_system_clock_is_a_clock() {
    fn assert_clock<T: domain::Clock + Default>() {}
    assert_clock::<SystemClock>();
}
