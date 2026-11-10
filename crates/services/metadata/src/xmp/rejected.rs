// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The rejected pick state, read from `xmp:Rating` (ADR 0044).

use super::model::{XmpName, XmpPacket, XmpValue};
use super::namespaces::XMP_BASIC;

impl XmpPacket {
    /// Whether the asset is rejected: `xmp:Rating` is a number below zero, as
    /// Lightroom, darktable and digiKam write `-1`. A missing rating, `0` to `5` and
    /// any text that is not a number are not rejected.
    #[must_use]
    pub fn is_rejected(&self) -> bool {
        let rating = XmpName::new(XMP_BASIC, "Rating");
        match self.property(&rating).map(|node| &node.value) {
            Some(XmpValue::Text(text)) => text.trim().parse::<f64>().is_ok_and(|value| value < 0.0),
            _ => false,
        }
    }
}
