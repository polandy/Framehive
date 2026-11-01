// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Job codes: a short label the user gives an import (IMPORT-006).

use std::fmt;

use naming::NameTemplate;
use thiserror::Error;

/// Most characters a job code may have (IMPORT-006).
pub const MAX_JOB_CODE_CHARACTERS: usize = 64;

/// A validated job code (glossary: Job code): the user's text without leading and
/// trailing whitespace, 1–64 characters, without `/` and control characters.
///
/// ```
/// use import::JobCode;
///
/// let job_code = JobCode::parse("  Smith wedding ")?;
/// assert_eq!(job_code.as_str(), "Smith wedding");
/// assert!(JobCode::parse("Smith/Jones").is_err());
/// # Ok::<(), import::JobCodeError>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobCode(String);

/// Why a text is not a valid job code (IMPORT-006).
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum JobCodeError {
    /// Nothing is left after removing the surrounding whitespace.
    #[error("a job code must not be empty")]
    Empty,
    /// More than [`MAX_JOB_CODE_CHARACTERS`] characters.
    #[error(
        "a job code has at most {MAX_JOB_CODE_CHARACTERS} characters, but this one has {characters}"
    )]
    TooLong {
        /// The number of characters after trimming.
        characters: usize,
    },
    /// A `/` would split the job code into folders.
    #[error("a job code must not contain \"/\"")]
    ContainsSlash,
    /// A control character (Unicode category Cc).
    #[error("a job code must not contain a control character")]
    ContainsControlCharacter,
}

impl JobCode {
    /// Takes the user's text as a job code, without its surrounding whitespace.
    ///
    /// # Errors
    ///
    /// Returns a [`JobCodeError`] naming the rule the trimmed text breaks.
    pub fn parse(text: &str) -> Result<Self, JobCodeError> {
        let text = text.trim();
        let characters = text.chars().count();
        if characters == 0 {
            return Err(JobCodeError::Empty);
        }
        if characters > MAX_JOB_CODE_CHARACTERS {
            return Err(JobCodeError::TooLong { characters });
        }
        if text.contains('/') {
            return Err(JobCodeError::ContainsSlash);
        }
        if text.chars().any(char::is_control) {
            return Err(JobCodeError::ContainsControlCharacter);
        }
        Ok(JobCode(text.to_owned()))
    }

    /// Returns the job code as text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for JobCode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// The name template contains `{job}`, but the import has no job code (IMPORT-006).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("the name template uses {{job}}, but no job code was given")]
pub struct MissingJobCode;

/// Checks that an import whose template contains `{job}` has a job code.
///
/// # Errors
///
/// Returns [`MissingJobCode`] when the template uses `{job}` and `job_code` is `None`.
pub fn require_job_code(
    template: &NameTemplate,
    job_code: Option<&JobCode>,
) -> Result<(), MissingJobCode> {
    if template.uses_job_code() && job_code.is_none() {
        return Err(MissingJobCode);
    }
    Ok(())
}
