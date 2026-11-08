// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The persisted sequence counters: their values, their rules and their port
//! (IMPORT-002, ADR 0029).

use std::fmt;
use std::path::PathBuf;

use domain::LocalDateTime;
use thiserror::Error;

use crate::{NameTemplate, SequenceKind, SequenceOverflow};

/// The machine-local calendar date the imported-today counter belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct CounterDate {
    year: u16,
    month: u8,
    day: u8,
}

impl CounterDate {
    /// The date of `time`.
    #[must_use]
    pub fn of(time: LocalDateTime) -> Self {
        CounterDate {
            year: time.year(),
            month: time.month(),
            day: time.day(),
        }
    }

    /// Parses `YYYY-MM-DD`, the form of `imported_today_date` (ADR 0029, section 2).
    ///
    /// ```
    /// use naming::CounterDate;
    ///
    /// let date = CounterDate::parse("2026-10-17").unwrap();
    /// assert_eq!(date.to_string(), "2026-10-17");
    /// assert_eq!(CounterDate::parse("2026-02-30"), None);
    /// assert_eq!(CounterDate::parse("2026-1-07"), None);
    /// ```
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        let [year, month, day] = <[&str; 3]>::try_from(text.split('-').collect::<Vec<_>>()).ok()?;
        let digits = |part: &str, length| {
            part.len() == length && part.bytes().all(|byte| byte.is_ascii_digit())
        };
        if !(digits(year, 4) && digits(month, 2) && digits(day, 2)) {
            return None;
        }
        let time = LocalDateTime::new(
            year.parse().ok()?,
            month.parse().ok()?,
            day.parse().ok()?,
            0,
            0,
            0,
        )
        .ok()?;
        Some(CounterDate::of(time))
    }
}

impl fmt::Display for CounterDate {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{:04}-{:02}-{:02}",
            self.year, self.month, self.day
        )
    }
}

/// The values of the persisted counters (ADR 0029, section 2). Each `*_last` is the
/// last number handed out; 0 means none yet.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct StoredCounters {
    /// The date `imported_today_last` belongs to, or `None` before the first import.
    pub imported_today_date: Option<CounterDate>,
    /// The last number of the imported-today counter on `imported_today_date`.
    pub imported_today_last: u32,
    /// The last number of the stored counter.
    pub stored_last: u32,
}

impl StoredCounters {
    /// The last number `kind` handed out on `today`: the imported-today counter starts
    /// again on a new day. Counters that are not persisted have none.
    #[must_use]
    pub fn last(&self, kind: SequenceKind, today: CounterDate) -> u32 {
        match kind {
            SequenceKind::ImportedToday if self.imported_today_date == Some(today) => {
                self.imported_today_last
            }
            SequenceKind::Stored => self.stored_last,
            SequenceKind::ImportedToday | SequenceKind::Session | SequenceKind::Letters => 0,
        }
    }

    /// The values after `kind` handed out numbers up to `last` on `today`. Counters
    /// that are not persisted change nothing.
    #[must_use]
    pub fn with_last(self, kind: SequenceKind, today: CounterDate, last: u32) -> Self {
        match kind {
            SequenceKind::ImportedToday => StoredCounters {
                imported_today_date: Some(today),
                imported_today_last: last,
                ..self
            },
            SequenceKind::Stored => StoredCounters {
                stored_last: last,
                ..self
            },
            SequenceKind::Session | SequenceKind::Letters => self,
        }
    }
}

/// The contiguous numbers of one import, `first..=last` (IMPORT-002, "Reservation").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SequenceRange {
    /// The number of the first asset in shooting order.
    pub first: u32,
    /// The number of the last asset.
    pub last: u32,
}

impl SequenceRange {
    /// The `count` numbers after `previous`, or `None` for no numbers at all.
    #[must_use]
    pub fn after(previous: u32, count: u32) -> Option<Self> {
        let last = previous.saturating_add(count);
        (count > 0).then(|| SequenceRange {
            first: previous.saturating_add(1),
            last,
        })
    }
}

/// Why the persisted counters cannot be read or stored (ADR 0029, section 3).
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum SequenceCountersError {
    /// The counter file holds something Framehive does not understand. It is not
    /// changed.
    #[error("the sequence counters in {} are broken at line {line}: {reason}; repair or delete the file", file.display())]
    Broken {
        /// The counter file.
        file: PathBuf,
        /// The line, counted from 1.
        line: usize,
        /// What is wrong with it.
        reason: String,
    },
    /// The counter file or its lock cannot be read or written.
    #[error("the sequence counters in {} cannot be read or written: {message}", file.display())]
    Unavailable {
        /// The file that failed.
        file: PathBuf,
        /// The error of the operating system.
        message: String,
    },
}

/// Port to the persisted sequence counters of this machine (ADR 0029); implemented by
/// `adapter-sequence-counters`.
///
/// Every implementation passes
/// [`testing::check_sequence_counters`](crate::testing::check_sequence_counters).
pub trait SequenceCounters {
    /// Reads the stored values without changing them. Missing values are 0.
    ///
    /// # Errors
    ///
    /// Returns a [`SequenceCountersError`] when the values cannot be read.
    fn read(&self) -> Result<StoredCounters, SequenceCountersError>;

    /// Reads the stored values, passes them to `change` and stores what it returns, in
    /// one step that no other process interleaves with. When `change` returns
    /// `None`, nothing is stored. Returns the values `change` was given.
    ///
    /// # Errors
    ///
    /// Returns a [`SequenceCountersError`] when the values cannot be read or stored;
    /// then the stored values are unchanged.
    fn update(
        &self,
        change: &mut dyn FnMut(StoredCounters) -> Option<StoredCounters>,
    ) -> Result<StoredCounters, SequenceCountersError>;
}

/// Why an import cannot get its sequence numbers; it does not start.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum SequenceError {
    /// The last number does not fit the template's sequence token.
    #[error(transparent)]
    Overflow(#[from] SequenceOverflow),
    /// The persisted counters cannot be read or stored.
    #[error(transparent)]
    Counters(#[from] SequenceCountersError),
}

/// The range a persisted counter would hand out to `count` assets on `today`, without
/// reserving it: for a dry run and the plan summary (IMPORT-002, "Reservation").
///
/// The counter is the one of the first of `templates` with a sequence token; the
/// caller makes sure the others with one share its kind (IMPORT-018). The range must
/// fit every template.
///
/// # Errors
///
/// Returns a [`SequenceError`] when the counters cannot be read or the range does not
/// fit a template.
pub fn preview_range(
    counters: &dyn SequenceCounters,
    templates: &[&NameTemplate],
    today: CounterDate,
    count: u32,
) -> Result<Option<SequenceRange>, SequenceError> {
    let Some(kind) = persisted_kind(templates) else {
        return Ok(None);
    };
    let range = SequenceRange::after(counters.read()?.last(kind, today), count);
    check_fits(templates, range)?;
    Ok(range)
}

/// Reserves the range of a persisted counter for `count` assets on `today` and stores
/// it, before the import copies its first file (IMPORT-002, "Reservation"). The
/// counter and the templates are as for [`preview_range`]; a range that does not fit
/// every template reserves nothing.
///
/// ```
/// use domain::LocalDateTime;
/// use naming::testing::MemorySequenceCounters;
/// use naming::{CounterDate, NameTemplate, SequenceRange, reserve_range};
///
/// let counters = MemorySequenceCounters::new();
/// let template = NameTemplate::parse("{seq_stored:4}.{ext}")?;
/// let today = CounterDate::of(LocalDateTime::new(2026, 10, 17, 9, 0, 0)?);
///
/// let range = reserve_range(&counters, &[&template], today, 3)?;
/// assert_eq!(range, Some(SequenceRange { first: 1, last: 3 }));
/// assert_eq!(counters.stored().stored_last, 3);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// # Errors
///
/// Returns a [`SequenceError`] when the counters cannot be read or stored, or the
/// range does not fit a template.
pub fn reserve_range(
    counters: &dyn SequenceCounters,
    templates: &[&NameTemplate],
    today: CounterDate,
    count: u32,
) -> Result<Option<SequenceRange>, SequenceError> {
    let Some(kind) = persisted_kind(templates) else {
        return Ok(None);
    };
    let mut outcome = Ok(None);
    counters.update(&mut |stored| {
        let range = SequenceRange::after(stored.last(kind, today), count);
        outcome = check_fits(templates, range).map(|()| range);
        let last = outcome.as_ref().ok().copied().flatten()?.last;
        Some(stored.with_last(kind, today, last))
    })?;
    Ok(outcome?)
}

fn persisted_kind(templates: &[&NameTemplate]) -> Option<SequenceKind> {
    templates
        .iter()
        .find_map(|template| template.sequence_kind())
        .filter(|kind| kind.is_persisted())
}

fn check_fits(
    templates: &[&NameTemplate],
    range: Option<SequenceRange>,
) -> Result<(), SequenceOverflow> {
    match range {
        Some(range) => templates
            .iter()
            .try_for_each(|template| template.check_sequence_range(range.last)),
        None => Ok(()),
    }
}
