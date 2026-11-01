// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The job codes used before, so that they can be offered again (IMPORT-006, "Recent
//! job codes").

use crate::job_code::JobCode;

/// Most job codes a [`RecentJobCodes`] list holds (IMPORT-006).
pub const MAX_RECENT_JOB_CODES: usize = 10;

/// The job codes used before (glossary: Recent job codes): the most recently used
/// first, each at most once, at most [`MAX_RECENT_JOB_CODES`]. Job codes are equal
/// when their texts are equal, case included.
///
/// ```
/// use import::{JobCode, RecentJobCodes};
///
/// let mut recent = RecentJobCodes::from_stored(["Harbour", "Smith wedding"]);
/// recent.mark_used(JobCode::parse("Smith wedding")?);
/// assert_eq!(recent.job_codes()[0].as_str(), "Smith wedding");
/// # Ok::<(), import::JobCodeError>(())
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RecentJobCodes {
    job_codes: Vec<JobCode>,
}

impl RecentJobCodes {
    /// An empty list.
    #[must_use]
    pub fn new() -> Self {
        RecentJobCodes::default()
    }

    /// Builds the list from stored job codes, most recently used first. Texts that are
    /// not valid job codes, and repeats of an earlier one, are left out; only the first
    /// [`MAX_RECENT_JOB_CODES`] that remain are kept.
    #[must_use]
    pub fn from_stored<I>(texts: I) -> Self
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        let mut job_codes: Vec<JobCode> = Vec::new();
        for text in texts {
            let Ok(job_code) = JobCode::parse(text.as_ref()) else {
                continue;
            };
            if !job_codes.contains(&job_code) {
                job_codes.push(job_code);
            }
        }
        job_codes.truncate(MAX_RECENT_JOB_CODES);
        RecentJobCodes { job_codes }
    }

    /// Records that `job_code` was used: it moves to the front, or is added there; the
    /// oldest job code is dropped when the list would grow beyond
    /// [`MAX_RECENT_JOB_CODES`].
    pub fn mark_used(&mut self, job_code: JobCode) {
        self.job_codes.retain(|known| *known != job_code);
        self.job_codes.insert(0, job_code);
        self.job_codes.truncate(MAX_RECENT_JOB_CODES);
    }

    /// The job codes, the most recently used first.
    #[must_use]
    pub fn job_codes(&self) -> &[JobCode] {
        &self.job_codes
    }
}
