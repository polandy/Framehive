// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Sequence numbers in file names (IMPORT-002).

use thiserror::Error;

use crate::NameTemplate;

/// Letters of the letter sequence, in order.
const LETTERS: &[u8; 26] = b"abcdefghijklmnopqrstuvwxyz";

/// Letter values: `a`…`z`, then `aa`…`zz` (IMPORT-002, "Letters").
const LAST_LETTER_NUMBER: u32 = 26 + 26 * 26;

/// Which counter a sequence token takes its number from (IMPORT-002, kind table).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SequenceKind {
    /// `{seq_today:N}`: all imports on the same machine-local day; persisted.
    ImportedToday,
    /// `{seq_stored:N}`: all imports ever on this machine; persisted.
    Stored,
    /// `{seq_session:N}`: one run of the application.
    Session,
    /// `{seq_letters}`: like [`SequenceKind::Session`], formatted as letters.
    Letters,
}

impl SequenceKind {
    /// Whether the counter outlives one run and is kept in the counter file
    /// (ADR 0029).
    #[must_use]
    pub fn is_persisted(self) -> bool {
        matches!(self, SequenceKind::ImportedToday | SequenceKind::Stored)
    }
}

/// How a sequence token formats its number.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SequenceFormat {
    /// Exactly this many digits, padded with zeros.
    Digits(usize),
    /// `a`…`z`, `aa`…`zz`.
    Letters,
}

impl SequenceFormat {
    /// The number formatted, or `None` when it is 0 or does not fit.
    pub(crate) fn format(self, number: u32) -> Option<String> {
        if number == 0 || number > self.largest() {
            return None;
        }
        Some(match self {
            SequenceFormat::Digits(width) => format!("{number:0>width$}"),
            SequenceFormat::Letters => letters(number),
        })
    }

    /// The largest number this format can show.
    fn largest(self) -> u32 {
        match self {
            // At most six digits (IMPORT-001), so the power fits a `u32`.
            SequenceFormat::Digits(width) => {
                (0..width).fold(1_u32, |power, _| power.saturating_mul(10)) - 1
            }
            SequenceFormat::Letters => LAST_LETTER_NUMBER,
        }
    }
}

fn letters(number: u32) -> String {
    let letter = |index: u32| char::from(LETTERS[index as usize % LETTERS.len()]);
    if number <= 26 {
        letter(number - 1).to_string()
    } else {
        let index = number - 27;
        format!("{}{}", letter(index / 26), letter(index % 26))
    }
}

/// The sequence token of a template: kind, format, and where it is written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SequenceToken {
    pub(crate) kind: SequenceKind,
    pub(crate) format: SequenceFormat,
    /// The token as written, for errors that name it.
    pub(crate) text: String,
}

/// The last number of an import's range does not fit the template's sequence token
/// (IMPORT-002, "Overflow").
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error(
    "{token} in the template {template:?} cannot show the number {last}{}",
    suggestion_text(.suggestion.as_deref())
)]
pub struct SequenceOverflow {
    /// The template as written.
    pub template: String,
    /// The sequence token as written.
    pub token: String,
    /// The last number the import needs.
    pub last: u32,
    /// The template with a sequence token that fits, if there is one.
    pub suggestion: Option<String>,
}

fn suggestion_text(suggestion: Option<&str>) -> String {
    match suggestion {
        Some(template) => format!("; use {template:?}"),
        None => String::new(),
    }
}

impl NameTemplate {
    /// The kind of the template's sequence token, if it has one.
    ///
    /// ```
    /// use naming::{NameTemplate, SequenceKind};
    ///
    /// let template = NameTemplate::parse("{seq_stored:4}.{ext}")?;
    /// assert_eq!(template.sequence_kind(), Some(SequenceKind::Stored));
    /// assert_eq!(NameTemplate::parse("{name}.{ext}")?.sequence_kind(), None);
    /// # Ok::<(), naming::TemplateError>(())
    /// ```
    #[must_use]
    pub fn sequence_kind(&self) -> Option<SequenceKind> {
        self.sequence().map(|(token, _)| token.kind)
    }

    /// Checks that the sequence token can show every number up to `last`, the last
    /// number of an import's range. A template without a sequence token always fits.
    ///
    /// ```
    /// use naming::NameTemplate;
    ///
    /// let template = NameTemplate::parse("{seq_stored:2}.{ext}")?;
    /// assert!(template.check_sequence_range(99).is_ok());
    /// let overflow = template.check_sequence_range(103).unwrap_err();
    /// assert_eq!(overflow.suggestion.as_deref(), Some("{seq_stored:3}.{ext}"));
    /// # Ok::<(), naming::TemplateError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// Returns a [`SequenceOverflow`] with a suggested template when `last` does not
    /// fit: one more digit, or for letters `{seq_session:N}` with enough digits.
    pub fn check_sequence_range(&self, last: u32) -> Result<(), SequenceOverflow> {
        let Some((token, start)) = self.sequence() else {
            return Ok(());
        };
        if last <= token.format.largest() {
            return Ok(());
        }
        let replacement = match token.format {
            SequenceFormat::Digits(width) => digits_token(token.kind, width + 1),
            SequenceFormat::Letters => digits_token(SequenceKind::Session, digit_count(last)),
        };
        let suggestion = replacement.map(|replacement| {
            let text = self.as_str();
            let end = start + token.text.len();
            format!("{}{replacement}{}", &text[..start], &text[end..])
        });
        Err(SequenceOverflow {
            template: self.as_str().to_owned(),
            token: token.text.clone(),
            last,
            suggestion,
        })
    }
}

/// The token of a digit sequence, or `None` beyond the six digits a token may have.
fn digits_token(kind: SequenceKind, width: usize) -> Option<String> {
    let name = match kind {
        SequenceKind::ImportedToday => "seq_today",
        SequenceKind::Stored => "seq_stored",
        SequenceKind::Session | SequenceKind::Letters => "seq_session",
    };
    (width <= crate::token::MAX_DIGITS).then(|| format!("{{{name}:{width}}}"))
}

fn digit_count(number: u32) -> usize {
    number.to_string().len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn digits_are_padded_and_bounded() {
        assert_eq!(SequenceFormat::Digits(3).format(7).as_deref(), Some("007"));
        assert_eq!(
            SequenceFormat::Digits(6).format(999_999).as_deref(),
            Some("999999")
        );
        assert_eq!(SequenceFormat::Digits(1).format(10), None);
        assert_eq!(SequenceFormat::Digits(1).format(0), None);
    }

    #[test]
    fn letters_run_from_a_to_zz() {
        let format = |number| SequenceFormat::Letters.format(number);

        assert_eq!(format(1).as_deref(), Some("a"));
        assert_eq!(format(26).as_deref(), Some("z"));
        assert_eq!(format(27).as_deref(), Some("aa"));
        assert_eq!(format(52).as_deref(), Some("az"));
        assert_eq!(format(53).as_deref(), Some("ba"));
        assert_eq!(format(702).as_deref(), Some("zz"));
        assert_eq!(format(703), None);
    }

    #[test]
    fn six_digits_have_no_larger_suggestion() {
        let template = NameTemplate::parse("{seq_stored:6}.{ext}").unwrap();

        let overflow = template.check_sequence_range(1_000_000).unwrap_err();

        assert_eq!(overflow.suggestion, None);
    }

    #[test]
    fn suggestion_replaces_the_token_not_an_escaped_copy() {
        let template = NameTemplate::parse("{{seq_stored:2}}_{seq_stored:2}.{ext}").unwrap();

        let overflow = template.check_sequence_range(100).unwrap_err();

        assert_eq!(
            overflow.suggestion.as_deref(),
            Some("{{seq_stored:2}}_{seq_stored:3}.{ext}")
        );
    }
}
