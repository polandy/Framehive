// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! A source on a file system of its own, such as an opened camera (ADR 0034).

use std::path::Path;

use files::ReadOnlyFileSystem;

use super::{ImportRun, ImportSettings};

impl<'a> ImportRun<'a> {
    /// Reads the source through `source_file_system` instead of the file system of
    /// the destinations, for a source on a device of its own, such as an opened
    /// camera (ADR 0034). The run only reads it. No destination can lie inside such a
    /// source, so the destinations are not checked against it.
    #[must_use]
    pub fn with_source(mut self, source_file_system: &'a dyn ReadOnlyFileSystem) -> Self {
        self.source_file_system = source_file_system;
        self.separate_source = true;
        self
    }

    /// Names the source in the import history as `name` instead of `settings.source`,
    /// for a source whose path says little, such as the root `/` of a camera
    /// (CLI-013, `camera:<model>`).
    #[must_use]
    pub fn with_source_name(mut self, name: &'a Path) -> Self {
        self.source_name = Some(name);
        self
    }

    /// The source the destinations are checked against: none for a source on a file
    /// system of its own.
    pub(super) fn checked_source<'s>(&self, settings: &ImportSettings<'s>) -> Option<&'s Path> {
        (!self.separate_source).then_some(settings.source)
    }
}
