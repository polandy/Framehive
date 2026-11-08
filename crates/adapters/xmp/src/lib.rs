// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The `xmp` adapter of Framehive: it implements the [`metadata::xmp::XmlSyntax`] port
//! with the XML tokenizer of `quick-xml` (ADR 0039, decision 1). Only apps depend on
//! it. See `README.md`.

#![forbid(unsafe_code)]

mod read;
mod write;

use metadata::xmp::{XmlEvent, XmlSyntax, XmlSyntaxError};

/// [`XmlSyntax`] with `quick-xml`: namespaces resolved, no document type declaration
/// and no entity but the five predefined ones.
#[derive(Debug, Clone, Copy, Default)]
pub struct QuickXml;

impl QuickXml {
    /// The tokenizer; it holds no state between texts.
    #[must_use]
    pub fn new() -> Self {
        QuickXml
    }
}

impl XmlSyntax for QuickXml {
    fn read(&self, text: &str) -> Result<Vec<XmlEvent>, XmlSyntaxError> {
        read::events(text)
    }

    fn write(&self, events: &[XmlEvent]) -> String {
        write::text(events)
    }
}
