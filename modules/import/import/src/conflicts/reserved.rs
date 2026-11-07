// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Target names that other import jobs reserved count as taken (IMPORT-015, "Reserved
//! target names", ADR 0036 §6).

use super::TargetResolver;
use crate::reserved_names::{ReservedNames, ReservedTargetNames};

/// The register of reserved names, and the names the resolving job reserved itself.
#[derive(Clone, Copy)]
pub(super) struct Reserved<'a> {
    register: &'a ReservedTargetNames,
    own: Option<&'a ReservedNames>,
}

impl<'a> TargetResolver<'a> {
    /// Counts the names in `register` that a job other than the one holding `own`
    /// reserved as taken by a file with other content.
    #[must_use]
    pub(crate) fn with_reserved_names(
        mut self,
        register: &'a ReservedTargetNames,
        own: Option<&'a ReservedNames>,
    ) -> Self {
        self.reserved = Some(Reserved { register, own });
        self
    }

    /// Whether another import job reserved `candidate` at this destination.
    pub(super) fn reserved_by_another_job(&self, candidate: &str) -> bool {
        self.reserved.is_some_and(|reserved| {
            reserved
                .register
                .taken(&self.destination.join(candidate), reserved.own)
        })
    }
}
