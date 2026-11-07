// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! One target resolver per destination of an import run: photos and videos resolve
//! their conflicts at their own destination (IMPORT-003, IMPORT-013).

use domain::AssetFileRole;

use super::{ImportRun, ImportSettings};
use crate::conflicts::TargetResolver;
use crate::reserved_names::{ReservedNames, ReservedTargetNames};

/// The register of reserved target names a run resolves against, and the names its
/// own job reserved, if any (IMPORT-015, "Reserved target names").
#[derive(Clone, Copy)]
pub(super) struct ReservedBy<'a> {
    register: &'a ReservedTargetNames,
    own: Option<&'a ReservedNames>,
}

impl<'a> ImportRun<'a> {
    /// Counts the target names in `register` that a job other than the one holding
    /// `own` reserved as taken (IMPORT-015.14).
    #[must_use]
    pub(crate) fn with_reserved_names(
        mut self,
        register: &'a ReservedTargetNames,
        own: Option<&'a ReservedNames>,
    ) -> Self {
        self.reserved_names = Some(ReservedBy { register, own });
        self
    }
}

/// The target resolver of the import destination, and of the video destination when
/// it is a folder of its own. A video destination that is the import destination
/// shares its resolver, so photos and videos claim their targets from one another.
pub(super) struct Resolvers<'a> {
    import_destination: TargetResolver<'a>,
    video_destination: Option<TargetResolver<'a>>,
}

impl<'a> Resolvers<'a> {
    /// The resolvers of the destinations of `settings`, through the ports of `run`.
    pub(super) fn new(run: &ImportRun<'a>, settings: &ImportSettings<'a>) -> Self {
        let resolver = |destination| {
            let resolver = TargetResolver::new(
                run.source_file_system,
                run.file_system,
                run.hasher,
                settings.source,
                destination,
                settings.policy,
            );
            match run.reserved_names {
                Some(reserved) => resolver.with_reserved_names(reserved.register, reserved.own),
                None => resolver,
            }
        };
        Resolvers {
            import_destination: resolver(settings.destination),
            video_destination: settings.separate_video_destination().map(resolver),
        }
    }

    /// The resolver of the destination of a file with `role`.
    pub(super) fn of(&mut self, role: AssetFileRole) -> &mut TargetResolver<'a> {
        match (role, &mut self.video_destination) {
            (AssetFileRole::Video, Some(resolver)) => resolver,
            _ => &mut self.import_destination,
        }
    }
}
