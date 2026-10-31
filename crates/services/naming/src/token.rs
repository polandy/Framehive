// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The tokens of a name template (IMPORT-001, token table).

use crate::date_format::DateFormat;
use crate::{TemplateError, TemplateErrorKind};

/// Most digits `{number:N}` and the sequence tokens may ask for.
const MAX_DIGITS: usize = 6;

/// A token between braces.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Token {
    /// `{date:<format>}`: the capture time of the asset.
    CaptureTime(DateFormat),
    /// `{imported:<format>}`: the time the import was queued.
    ImportTime(DateFormat),
    /// `{name}`: the original base name.
    BaseName(Case),
    /// `{ext}`: the original extension, without the dot.
    Extension(Case),
    /// `{number}` or `{number:N}`: the image number.
    ImageNumber {
        /// `N`, or `None` for all trailing digits.
        digits: Option<usize>,
        /// The token as written, for the error that names it.
        text: String,
    },
    /// A sequence token (IMPORT-002), kept as written.
    Sequence { text: String },
}

/// The case option of `{name}` and `{ext}`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Case {
    Original,
    Lower,
    Upper,
}

impl Case {
    pub(crate) fn apply(self, value: &str) -> String {
        match self {
            Case::Original => value.to_owned(),
            Case::Lower => value.to_lowercase(),
            Case::Upper => value.to_uppercase(),
        }
    }
}

impl Token {
    /// Parses the `content` between the braces of a token whose `{` is at `column`.
    pub(crate) fn parse(content: &str, column: usize) -> Result<Self, TemplateError> {
        let text = format!("{{{content}}}");
        let (name, option) = match content.split_once(':') {
            Some((name, option)) => (name, Some(option)),
            None => (content, None),
        };
        let unknown_option = || {
            let token = text.clone();
            TemplateError::new(column, TemplateErrorKind::UnknownOption { token })
        };
        match (name, option) {
            ("date" | "imported", Some("")) | ("seq_letters", Some(_)) => Err(unknown_option()),
            ("date" | "imported", Some(format)) => {
                // The format starts after `{`, the name and `:`.
                let format = DateFormat::parse(format, column + name.len() + 2)?;
                Ok(date_token(name, format))
            }
            ("date" | "imported" | "seq_today" | "seq_stored" | "seq_session", None) => Err(
                TemplateError::new(column, TemplateErrorKind::MissingOption { token: text }),
            ),
            ("name", option) => Ok(Token::BaseName(case(option).ok_or_else(unknown_option)?)),
            ("ext", option) => Ok(Token::Extension(case(option).ok_or_else(unknown_option)?)),
            ("number", None) => Ok(Token::ImageNumber { digits: None, text }),
            ("number", Some(option)) => {
                let digits = Some(digit_count(option).ok_or_else(unknown_option)?);
                Ok(Token::ImageNumber { digits, text })
            }
            ("seq_today" | "seq_stored" | "seq_session", Some(option)) => {
                digit_count(option).ok_or_else(unknown_option)?;
                Ok(Token::Sequence { text })
            }
            ("seq_letters", None) => Ok(Token::Sequence { text }),
            _ => Err(TemplateError::new(
                column,
                TemplateErrorKind::UnknownToken { token: text },
            )),
        }
    }
}

fn date_token(name: &str, format: DateFormat) -> Token {
    if name == "date" {
        Token::CaptureTime(format)
    } else {
        Token::ImportTime(format)
    }
}

fn case(option: Option<&str>) -> Option<Case> {
    match option {
        None => Some(Case::Original),
        Some("lower") => Some(Case::Lower),
        Some("upper") => Some(Case::Upper),
        Some(_) => None,
    }
}

/// The `N` of `{number:N}` and the sequence tokens: one digit, 1–6.
fn digit_count(option: &str) -> Option<usize> {
    match option.as_bytes() {
        [digit @ b'1'..=b'9'] => Some(usize::from(digit - b'0')).filter(|&n| n <= MAX_DIGITS),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sequence_tokens_need_a_digit_count_from_one_to_six() {
        let kind = |content| Token::parse(content, 1).unwrap_err().kind;

        assert!(matches!(
            kind("seq_today"),
            TemplateErrorKind::MissingOption { .. }
        ));
        assert!(matches!(
            kind("seq_stored:7"),
            TemplateErrorKind::UnknownOption { .. }
        ));
        assert!(matches!(
            kind("seq_letters:2"),
            TemplateErrorKind::UnknownOption { .. }
        ));
        assert!(matches!(
            kind("date:"),
            TemplateErrorKind::UnknownOption { .. }
        ));
        assert!(matches!(kind(""), TemplateErrorKind::UnknownToken { .. }));
    }
}
