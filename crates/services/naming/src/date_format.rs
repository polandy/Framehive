// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The date formats of `{date:…}` and `{imported:…}` (IMPORT-001, "Date formats").

use std::fmt::Write as _;

use crate::{LocalDateTime, TemplateError, TemplateErrorKind};

/// A parsed date format: literal text and date codes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DateFormat(Vec<DatePart>);

#[derive(Debug, Clone, PartialEq, Eq)]
enum DatePart {
    Literal(char),
    Year,
    YearOfCentury,
    Month,
    Day,
    DayOfYear,
    Hour,
    Minute,
    Second,
}

impl DateFormat {
    /// Parses `format`, whose first character is at `column` of the template.
    pub(crate) fn parse(format: &str, column: usize) -> Result<Self, TemplateError> {
        let mut parts = Vec::new();
        let mut chars = format.chars().zip(column..);
        while let Some((character, column)) = chars.next() {
            let part = match character {
                '%' => code(chars.next().map(|(next, _)| next), column)?,
                '/' => {
                    return Err(TemplateError::new(
                        column,
                        TemplateErrorKind::SlashInDateFormat,
                    ));
                }
                literal => DatePart::Literal(literal),
            };
            parts.push(part);
        }
        Ok(DateFormat(parts))
    }

    /// Appends `time` in this format to `output`.
    pub(crate) fn render(&self, time: LocalDateTime, output: &mut String) {
        for part in &self.0 {
            // Writing to a String cannot fail.
            let _ = match part {
                DatePart::Literal(character) => {
                    output.push(*character);
                    Ok(())
                }
                DatePart::Year => write!(output, "{:04}", time.year()),
                DatePart::YearOfCentury => write!(output, "{:02}", time.year() % 100),
                DatePart::Month => write!(output, "{:02}", time.month()),
                DatePart::Day => write!(output, "{:02}", time.day()),
                DatePart::DayOfYear => write!(output, "{:03}", time.day_of_year()),
                DatePart::Hour => write!(output, "{:02}", time.hour()),
                DatePart::Minute => write!(output, "{:02}", time.minute()),
                DatePart::Second => write!(output, "{:02}", time.second()),
            };
        }
    }
}

/// The date part for the code letter after a `%` at `column`.
fn code(letter: Option<char>, column: usize) -> Result<DatePart, TemplateError> {
    Ok(match letter {
        Some('Y') => DatePart::Year,
        Some('y') => DatePart::YearOfCentury,
        Some('m') => DatePart::Month,
        Some('d') => DatePart::Day,
        Some('j') => DatePart::DayOfYear,
        Some('H') => DatePart::Hour,
        Some('M') => DatePart::Minute,
        Some('S') => DatePart::Second,
        Some('%') => DatePart::Literal('%'),
        other => {
            let code = other.map_or_else(|| "%".to_owned(), |letter| format!("%{letter}"));
            return Err(TemplateError::new(
                column,
                TemplateErrorKind::UnknownDateCode { code },
            ));
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_a_lone_percent_sign_at_the_end() {
        let error = DateFormat::parse("%Y%", 7).unwrap_err();

        assert_eq!(error.column, 9);
        assert_eq!(
            error.kind,
            TemplateErrorKind::UnknownDateCode {
                code: "%".to_owned()
            }
        );
    }
}
