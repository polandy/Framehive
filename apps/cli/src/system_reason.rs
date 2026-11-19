// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! A reason from the operating system in plain words (CLI-001, "Reasons from the
//! operating system").

use std::fmt;
use std::io::{self, ErrorKind};

/// Shows an I/O error as CLI-001 names a reason from the operating system: in plain
/// words and without its error number. An error that does not come from the operating
/// system shows its own message.
pub(crate) struct PlainReason<'a>(pub(crate) &'a io::Error);

impl fmt::Display for PlainReason<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0.raw_os_error().is_none() {
            return write!(formatter, "{}", self.0);
        }
        match self.0.kind() {
            ErrorKind::NotFound => formatter.write_str("it does not exist"),
            ErrorKind::PermissionDenied => formatter.write_str("permission denied"),
            ErrorKind::StorageFull => formatter.write_str("the disk is full"),
            ErrorKind::ReadOnlyFilesystem => formatter.write_str("the file system is read-only"),
            ErrorKind::NotADirectory => formatter.write_str("a part of the path is not a folder"),
            _ => formatter.write_str(&without_error_number(&self.0.to_string())),
        }
    }
}

/// The text of an operating system error with a lower-case first letter and without
/// ` (os error <n>)`.
fn without_error_number(text: &str) -> String {
    let bare = text
        .rsplit_once(" (os error ")
        .map_or(text, |(bare, _)| bare);
    let mut characters = bare.chars();
    characters
        .next()
        .map(|first| first.to_lowercase().chain(characters).collect())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plain(error: &io::Error) -> String {
        PlainReason(error).to_string()
    }

    /// spec: CLI-019.2
    #[test]
    fn common_reasons_are_in_plain_words() {
        let os_error = |kind| {
            [2, 13, 28, 30, 20]
                .into_iter()
                .map(io::Error::from_raw_os_error)
                .find(|error| error.kind() == kind)
        };
        let cases = [
            (ErrorKind::NotFound, "it does not exist"),
            (ErrorKind::PermissionDenied, "permission denied"),
            (ErrorKind::StorageFull, "the disk is full"),
            (
                ErrorKind::ReadOnlyFilesystem,
                "the file system is read-only",
            ),
            (
                ErrorKind::NotADirectory,
                "a part of the path is not a folder",
            ),
        ];
        for (kind, expected) in cases {
            let error = os_error(kind);
            assert_eq!(
                error.as_ref().map(plain).as_deref(),
                Some(expected),
                "{kind:?}"
            );
        }
    }

    /// spec: CLI-019.3
    #[test]
    fn another_reason_loses_its_error_number() {
        // EISDIR is 21 on Linux and macOS.
        let error = io::Error::from_raw_os_error(21);

        assert_eq!(plain(&error), "is a directory");
    }

    #[test]
    fn an_error_of_framehive_keeps_its_message() {
        let error = io::Error::new(ErrorKind::NotFound, "Deleted by the test");

        assert_eq!(plain(&error), "Deleted by the test");
    }
}
