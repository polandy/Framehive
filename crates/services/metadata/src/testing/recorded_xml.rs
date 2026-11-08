// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! A test double of the [`XmlSyntax`] port that needs no XML library.

use std::collections::BTreeMap;
use std::sync::{Mutex, MutexGuard, PoisonError};

use crate::xmp::{XmlEvent, XmlSyntax, XmlSyntaxError, XmlSyntaxErrorKind};

/// An [`XmlSyntax`] that reads back only the text it wrote; any other text is
/// malformed. It keeps tests free of an XML adapter, which a service or a module may
/// not depend on (architecture §3); `adapter-xmp` tests the real XML.
#[derive(Debug, Default)]
pub struct RecordedXml(Mutex<BTreeMap<String, Vec<XmlEvent>>>);

impl RecordedXml {
    /// A double that has written nothing yet.
    #[must_use]
    pub fn new() -> Self {
        RecordedXml::default()
    }

    fn written(&self) -> MutexGuard<'_, BTreeMap<String, Vec<XmlEvent>>> {
        // A test that panicked while holding the lock left a complete map behind.
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl XmlSyntax for RecordedXml {
    fn read(&self, text: &str) -> Result<Vec<XmlEvent>, XmlSyntaxError> {
        self.written()
            .get(text.trim_end())
            .cloned()
            .ok_or_else(|| XmlSyntaxError {
                offset: 0,
                kind: XmlSyntaxErrorKind::Malformed("not written by this double".to_owned()),
            })
    }

    fn write(&self, events: &[XmlEvent]) -> String {
        let mut written = self.written();
        let text = format!("<recorded {}/>", written.len());
        written.insert(text.clone(), events.to_vec());
        text
    }
}
