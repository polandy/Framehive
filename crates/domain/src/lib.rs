// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Pure business types of Framehive.
//!
//! This crate has no I/O: no file system, no network, no wall clock, no threads
//! (architecture §3). Everything that needs the outside world goes through ports.

#![forbid(unsafe_code)]

mod asset_file;
mod capture_time;
mod checksum;
mod clock;
mod date_time;
mod rating;
pub mod testing;
mod time;

pub use asset_file::AssetFileRole;
pub use capture_time::CaptureTime;
pub use checksum::{Checksum, ChecksumError};
pub use clock::Clock;
pub use date_time::{DateTimeError, LocalDateTime};
pub use rating::{Rating, RatingError};
pub use time::{TimeError, Timestamp, UtcOffset};
