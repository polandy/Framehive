// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Fuzz target `capture_time`: any bytes go to the EXIF parser (ADR 0037 §2).
//!
//! The parser is a pure function of the bytes, so no port is needed. Its result does
//! not matter; a panic, an overflow or an allocation above libFuzzer's limit is a
//! finding (CORE-005.9, CORE-005.12).

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|content: &[u8]| {
    let _ = metadata::capture_time_from_bytes(content);
});
