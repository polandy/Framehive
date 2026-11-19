// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The mount table override of the guided import (ADR 0050).

use std::ffi::OsString;

use crate::guided::memory_cards_from;

/// The usage error when only one of the two values is given.
const ONLY_ONE: &str =
    "FRAMEHIVE_MOUNTINFO and FRAMEHIVE_MEDIA_FOLDERS are set together or not at all";

/// Only one of the two is a usage error, so a typo never reads the real mount table.
#[test]
fn only_one_of_the_two_is_refused() {
    let given = || Some(OsString::from("/tmp/framehive-test"));

    for (mountinfo, folders) in [(given(), None), (None, given())] {
        let message = memory_cards_from(mountinfo, folders).err();

        assert_eq!(message.as_deref(), Some(ONLY_ONE));
    }
}

#[test]
fn both_or_neither_are_accepted() {
    let given = || Some(OsString::from("/tmp/framehive-test"));

    assert!(memory_cards_from(given(), given()).is_ok());
    assert!(memory_cards_from(None, None).is_ok());
}
