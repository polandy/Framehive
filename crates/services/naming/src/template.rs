// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Parsing and validation of name templates (IMPORT-001, "Validation").

use crate::sequence::SequenceToken;
use crate::token::Token;
use crate::{TemplateError, TemplateErrorKind};

/// A validated name template: text that derives an imported file's folders and name
/// (IMPORT-001).
///
/// `/` separates folders, the last segment is the file name, and it ends with
/// `.{ext}`. A template is validated once, when it is parsed; rendering it then fails
/// only for values that cannot be named (see [`RenderError`](crate::RenderError)).
///
/// ```
/// use naming::NameTemplate;
///
/// let template = NameTemplate::parse("{date:%Y}/{date:%Y-%m-%d}/{name}.{ext}")?;
/// assert_eq!(template.as_str(), "{date:%Y}/{date:%Y-%m-%d}/{name}.{ext}");
/// assert!(NameTemplate::parse("{date:%Y}/{camera}.{ext}").is_err());
/// # Ok::<(), naming::TemplateError>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NameTemplate {
    text: String,
    segments: Vec<Vec<Part>>,
    /// Byte offset of the sequence token in `text`, if there is one.
    sequence_start: Option<usize>,
}

/// One piece of a segment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Part {
    Literal(String),
    Token(Token),
}

/// A character of the template after resolving `{{`, `}}` and tokens.
enum Lexeme {
    Text(char),
    Separator,
    Token(Token),
}

impl NameTemplate {
    /// Parses and validates a name template.
    ///
    /// # Errors
    ///
    /// Returns a [`TemplateError`] with the column of the first rule the template
    /// violates.
    pub fn parse(text: &str) -> Result<Self, TemplateError> {
        let Scanned {
            lexemes,
            sequence_start,
        } = scan(text)?;
        let segments = split_into_segments(lexemes)?;
        Ok(NameTemplate {
            text: text.to_owned(),
            segments,
            sequence_start,
        })
    }

    /// Returns the template as it was written.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.text
    }

    /// Whether the template contains `{job}`, so an import with it needs a job code
    /// (IMPORT-006).
    ///
    /// ```
    /// use naming::NameTemplate;
    ///
    /// assert!(NameTemplate::parse("{job}/{name}.{ext}")?.uses_job_code());
    /// assert!(!NameTemplate::parse("{{job}}/{name}.{ext}")?.uses_job_code());
    /// # Ok::<(), naming::TemplateError>(())
    /// ```
    #[must_use]
    pub fn uses_job_code(&self) -> bool {
        self.segments
            .iter()
            .flatten()
            .any(|part| matches!(part, Part::Token(Token::JobCode)))
    }

    pub(crate) fn segments(&self) -> &[Vec<Part>] {
        &self.segments
    }

    /// The sequence token and its byte offset in the template text.
    pub(crate) fn sequence(&self) -> Option<(&SequenceToken, usize)> {
        let token = self.segments.iter().flatten().find_map(|part| match part {
            Part::Token(Token::Sequence(token)) => Some(token),
            _ => None,
        })?;
        Some((token, self.sequence_start?))
    }
}

/// The lexemes of a template with their columns, and the byte offset of its sequence
/// token.
struct Scanned {
    lexemes: Vec<(Lexeme, usize)>,
    sequence_start: Option<usize>,
}

/// Resolves escaped braces and tokens, in order of their column.
fn scan(text: &str) -> Result<Scanned, TemplateError> {
    let chars: Vec<char> = text.chars().collect();
    let mut lexemes = Vec::new();
    let mut sequence_start = None;
    let mut index = 0;
    let mut byte_offset = 0;
    while let Some(&character) = chars.get(index) {
        let column = index + 1;
        let next = chars.get(index + 1).copied();
        let (lexeme, length) = match character {
            '{' | '}' if next == Some(character) => (Lexeme::Text(character), 2),
            '}' => {
                return Err(TemplateError::new(
                    column,
                    TemplateErrorKind::UnbalancedBrace,
                ));
            }
            '{' => {
                let content = token_content(&chars, index)?;
                let token = Token::parse(&content, column)?;
                check_single_sequence(&token, &mut sequence_start, byte_offset, column)?;
                (Lexeme::Token(token), content.chars().count() + 2)
            }
            '/' => (Lexeme::Separator, 1),
            other => (Lexeme::Text(other), 1),
        };
        lexemes.push((lexeme, column));
        byte_offset += chars[index..index + length]
            .iter()
            .map(|c| c.len_utf8())
            .sum::<usize>();
        index += length;
    }
    Ok(Scanned {
        lexemes,
        sequence_start,
    })
}

/// The text between the `{` at `start` and its `}`.
fn token_content(chars: &[char], start: usize) -> Result<String, TemplateError> {
    let mut content = String::new();
    for (index, &character) in chars.iter().enumerate().skip(start + 1) {
        match character {
            '}' => return Ok(content),
            '{' => {
                return Err(TemplateError::new(
                    index + 1,
                    TemplateErrorKind::UnbalancedBrace,
                ));
            }
            other => content.push(other),
        }
    }
    Err(TemplateError::new(
        start + 1,
        TemplateErrorKind::UnbalancedBrace,
    ))
}

fn check_single_sequence(
    token: &Token,
    sequence_start: &mut Option<usize>,
    byte_offset: usize,
    column: usize,
) -> Result<(), TemplateError> {
    let Token::Sequence(sequence) = token else {
        return Ok(());
    };
    if sequence_start.is_some() {
        let token = sequence.text.clone();
        return Err(TemplateError::new(
            column,
            TemplateErrorKind::SecondSequenceToken { token },
        ));
    }
    *sequence_start = Some(byte_offset);
    Ok(())
}

/// Splits the lexemes at `/` and checks the segments and the final `.{ext}`.
fn split_into_segments(lexemes: Vec<(Lexeme, usize)>) -> Result<Vec<Vec<Part>>, TemplateError> {
    let mut segments = Vec::new();
    let mut current = Vec::new();
    let mut start = 1;
    for (lexeme, column) in lexemes {
        match lexeme {
            Lexeme::Separator => {
                check_segment(&current, start, segments.is_empty())?;
                segments.push(std::mem::take(&mut current));
                start = column + 1;
            }
            Lexeme::Text(character) => push_text(&mut current, character),
            Lexeme::Token(token) => current.push(Part::Token(token)),
        }
    }
    // Only a `/` can make the first segment a leading slash; an empty template is
    // simply an empty segment.
    check_segment(&current, start, false)?;
    check_extension(&current, start)?;
    segments.push(current);
    Ok(segments)
}

fn push_text(parts: &mut Vec<Part>, character: char) {
    if let Some(Part::Literal(text)) = parts.last_mut() {
        text.push(character);
    } else {
        parts.push(Part::Literal(character.to_string()));
    }
}

fn check_segment(parts: &[Part], column: usize, is_first: bool) -> Result<(), TemplateError> {
    match parts {
        [] if is_first => Err(TemplateErrorKind::LeadingSlash),
        [] => Err(TemplateErrorKind::EmptySegment),
        [Part::Literal(text)] if text == "." || text == ".." => {
            Err(TemplateErrorKind::DotSegment {
                segment: text.clone(),
            })
        }
        _ => Ok(()),
    }
    .map_err(|kind| TemplateError::new(column, kind))
}

fn check_extension(last: &[Part], column: usize) -> Result<(), TemplateError> {
    match last {
        [.., Part::Literal(text), Part::Token(Token::Extension(_))] if text.ends_with('.') => {
            Ok(())
        }
        _ => Err(TemplateError::new(
            column,
            TemplateErrorKind::MissingExtension,
        )),
    }
}
