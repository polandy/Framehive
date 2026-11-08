// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Where the sequence numbers of an import come from (IMPORT-002, "Reservation").

use std::sync::atomic::{AtomicU32, Ordering};

use naming::{
    CounterDate, NameTemplate, SequenceCounters, SequenceError, SequenceKind, SequenceRange,
    preview_range, reserve_range,
};
use thiserror::Error;

/// The session counter: the numbers of `{seq_session:N}` and `{seq_letters}` in one
/// run of the application (IMPORT-002). It starts at 1 and is never stored.
///
/// The application makes one and passes it to every import it runs, so the imports
/// of one run continue each other's numbers. It can be shared with the import jobs
/// on the job queue's worker thread (IMPORT-015).
#[derive(Debug, Default)]
pub struct SessionSequence {
    last: AtomicU32,
}

impl SessionSequence {
    /// A session counter that has handed out no number yet.
    #[must_use]
    pub fn new() -> Self {
        SessionSequence::default()
    }

    /// The last number handed out, or 0.
    #[must_use]
    pub fn last(&self) -> u32 {
        self.last.load(Ordering::SeqCst)
    }
}

/// Whether a sequence range is only shown or also reserved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Reservation {
    /// A dry run or a plan summary: show the numbers, reserve nothing.
    Preview,
    /// The import starts: reserve the numbers before the first file is copied.
    Reserve,
}

/// The name template and the video name template have sequence tokens of different
/// kinds, so their assets cannot share one numbering (IMPORT-018).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error(
    "the video template uses {} but the name template uses {}; use one sequence kind in both",
    .video.token(),
    .name.token()
)]
pub struct MixedSequenceKinds {
    /// The kind of the video name template's sequence token.
    pub video: SequenceKind,
    /// The kind of the name template's sequence token.
    pub name: SequenceKind,
}

impl MixedSequenceKinds {
    /// Checks that `template` and `video_template` do not number by different kinds.
    pub(crate) fn check(
        template: &NameTemplate,
        video_template: &NameTemplate,
    ) -> Result<(), MixedSequenceKinds> {
        match (template.sequence_kind(), video_template.sequence_kind()) {
            (Some(name), Some(video)) if name != video => Err(MixedSequenceKinds { video, name }),
            _ => Ok(()),
        }
    }
}

/// The range of `count` assets for `templates` on `today`, from the persisted
/// counters or the session counter. The templates share the kind of their sequence
/// tokens, and the range must fit each of them. `None` when no template has a
/// sequence token or there are no assets.
pub(crate) fn sequence_range(
    counters: &dyn SequenceCounters,
    session: &SessionSequence,
    templates: &[&NameTemplate],
    today: CounterDate,
    count: u32,
    reservation: Reservation,
) -> Result<Option<SequenceRange>, SequenceError> {
    let Some(kind) = templates
        .iter()
        .find_map(|template| template.sequence_kind())
    else {
        return Ok(None);
    };
    if kind.is_persisted() {
        return match reservation {
            Reservation::Preview => preview_range(counters, templates, today, count),
            Reservation::Reserve => reserve_range(counters, templates, today, count),
        };
    }
    let range = SequenceRange::after(session.last(), count);
    if let Some(range) = range {
        for template in templates {
            template.check_sequence_range(range.last)?;
        }
        if reservation == Reservation::Reserve {
            session.last.store(range.last, Ordering::SeqCst);
        }
    }
    Ok(range)
}
