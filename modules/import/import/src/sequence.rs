// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Where the sequence numbers of an import come from (IMPORT-002, "Reservation").

use std::sync::atomic::{AtomicU32, Ordering};

use naming::{
    CounterDate, NameTemplate, SequenceCounters, SequenceError, SequenceRange, preview_range,
    reserve_range,
};

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

/// The range of `count` assets for `template` on `today`, from the persisted
/// counters or the session counter. `None` when the template has no sequence token
/// or there are no assets.
pub(crate) fn sequence_range(
    counters: &dyn SequenceCounters,
    session: &SessionSequence,
    template: &NameTemplate,
    today: CounterDate,
    count: u32,
    reservation: Reservation,
) -> Result<Option<SequenceRange>, SequenceError> {
    let Some(kind) = template.sequence_kind() else {
        return Ok(None);
    };
    if kind.is_persisted() {
        return match reservation {
            Reservation::Preview => preview_range(counters, template, today, count),
            Reservation::Reserve => reserve_range(counters, template, today, count),
        };
    }
    let range = SequenceRange::after(session.last(), count);
    if let Some(range) = range {
        template.check_sequence_range(range.last)?;
        if reservation == Reservation::Reserve {
            session.last.store(range.last, Ordering::SeqCst);
        }
    }
    Ok(range)
}
