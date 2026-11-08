// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The errors of a verified copy that are built outside the copy itself.

#![allow(clippy::panic, reason = "a test fails by panicking")]

use std::path::Path;

use files::{CopyError, CopyErrorKind, CopyStep};

#[test]
fn unfinished_copy_fails_the_verify_step_of_its_target() {
    let target = Path::new("/photos/2026/DSCF0001.RAF");

    let error = CopyError::unfinished(target);

    assert_eq!(error.step, CopyStep::Verify);
    let CopyErrorKind::Io { path, error: cause } = &error.kind else {
        panic!("expected an I/O error, got {:?}", error.kind);
    };
    assert_eq!(path, target);
    assert_eq!(cause.to_string(), "the copy did not finish");
    assert!(error.cleanup.is_none());
}
