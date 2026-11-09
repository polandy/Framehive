// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The Keep RAW marker, `framehive:KeepRaw` (LIBRARY-002, ADR 0042).

use super::edit::wrong_form;
use super::error::XmpEditError;
use super::model::{XmpName, XmpPacket, XmpValue};
use super::namespaces::FRAMEHIVE;

/// The XMP Boolean true.
const TRUE: &str = "True";

impl XmpPacket {
    /// Whether the asset is marked Keep RAW: `framehive:KeepRaw` is the XMP Boolean
    /// true, in any case. A missing property, `False` or any other value is unmarked.
    #[must_use]
    pub fn keep_raw(&self) -> bool {
        match self.property(&keep_raw_name()).map(|node| &node.value) {
            Some(XmpValue::Text(text)) => text.eq_ignore_ascii_case(TRUE),
            _ => false,
        }
    }

    /// Marks the asset Keep RAW by writing `framehive:KeepRaw` as `True`, or unmarks it
    /// by removing the property, so an unmarked sidecar gains nothing.
    ///
    /// # Errors
    ///
    /// [`XmpEditError::WrongForm`] when `framehive:KeepRaw` holds a struct or an array;
    /// the packet then stays unchanged.
    pub fn set_keep_raw(&mut self, marked: bool) -> Result<(), XmpEditError> {
        let name = keep_raw_name();
        if marked {
            return self.set_text(&name, TRUE);
        }
        match self.property(&name).map(|node| &node.value) {
            None | Some(XmpValue::Text(_) | XmpValue::Uri(_)) => {
                self.properties.remove(&name);
                Ok(())
            }
            Some(_) => Err(wrong_form(&name, "a simple value")),
        }
    }
}

fn keep_raw_name() -> XmpName {
    XmpName::new(FRAMEHIVE, "KeepRaw")
}
