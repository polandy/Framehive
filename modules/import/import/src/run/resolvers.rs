// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! One target resolver per destination of an import run: photos and videos resolve
//! their conflicts at their own destination (IMPORT-003, IMPORT-013).

use domain::AssetFileRole;

use super::{ImportRun, ImportSettings};
use crate::conflicts::TargetResolver;

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
            TargetResolver::new(
                run.file_system,
                run.hasher,
                settings.source,
                destination,
                settings.policy,
            )
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
