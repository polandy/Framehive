// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Why a file has no capture time (CORE-005).

use thiserror::Error;

/// Why a file has no capture time, reported with the file when it is dated by its
/// modification time instead (CORE-005).
///
/// ```
/// use metadata::NoCaptureTime;
///
/// let reason = NoCaptureTime::Invalid {
///     value: "2026:13:40 25:00:00".to_owned(),
/// };
/// assert_eq!(reason.to_string(), "invalid capture time: 2026:13:40 25:00:00");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum NoCaptureTime {
    /// The format is not one whose EXIF is read yet, such as a video, CR3 or HEIF.
    #[error("format not read")]
    FormatNotRead,
    /// The file has no `DateTimeOriginal`, or its value means "unknown" (blank or all
    /// zeros).
    #[error("no capture time")]
    Missing,
    /// `DateTimeOriginal` is not a valid date and time.
    #[error("invalid capture time: {value}")]
    Invalid {
        /// The value as recorded, with control characters escaped.
        value: String,
    },
    /// The file is damaged: truncated, or with pointers outside the file or in a loop.
    #[error("damaged: {what}")]
    Damaged {
        /// What is wrong, e.g. `IFD at offset 8 is read twice`.
        what: String,
    },
}

impl NoCaptureTime {
    /// A [`NoCaptureTime::Damaged`] that says `what` is wrong.
    pub(crate) fn damaged(what: impl Into<String>) -> Self {
        NoCaptureTime::Damaged { what: what.into() }
    }
}
