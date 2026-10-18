// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Pure business types of Framehive.
//!
//! This crate has no I/O: no file system, no network, no wall clock, no threads
//! (architecture §3). Everything that needs the outside world goes through ports.

#![forbid(unsafe_code)]

mod rating;

pub use rating::{Rating, RatingError};
