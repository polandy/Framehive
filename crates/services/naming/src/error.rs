// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

use thiserror::Error;

/// Why a name template was rejected, and where (IMPORT-001, "Validation").
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("column {column}: {kind}")]
pub struct TemplateError {
    /// Column of the error, counted in characters from 1.
    pub column: usize,
    /// What is wrong.
    pub kind: TemplateErrorKind,
}

impl TemplateError {
    pub(crate) fn new(column: usize, kind: TemplateErrorKind) -> Self {
        TemplateError { column, kind }
    }
}

/// The rule a rejected name template violates.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum TemplateErrorKind {
    /// A `{` without its `}`, or a `}` without its `{`.
    #[error("unbalanced brace; write {{{{ or }}}} for a literal brace")]
    UnbalancedBrace,
    /// The token name is not known.
    #[error("unknown token {token}")]
    UnknownToken {
        /// The token as written, with its braces.
        token: String,
    },
    /// The token is known, but its option is not.
    #[error("unknown option in {token}")]
    UnknownOption {
        /// The token as written, with its braces.
        token: String,
    },
    /// The token needs an option, e.g. the date format of `{date:…}`.
    #[error("{token} needs an option, e.g. {{date:%Y-%m-%d}}")]
    MissingOption {
        /// The token as written, with its braces.
        token: String,
    },
    /// A date format contains a code that is not known.
    #[error("unknown date code {code}")]
    UnknownDateCode {
        /// The code as written, e.g. `%B`.
        code: String,
    },
    /// A date format contains `/`; folders come from separate segments.
    #[error("a date format must not contain /; put each folder in its own segment")]
    SlashInDateFormat,
    /// The template has a second sequence token.
    #[error("only one sequence token is allowed, but {token} is another one")]
    SecondSequenceToken {
        /// The second sequence token as written, with its braces.
        token: String,
    },
    /// The last segment does not end with `.{ext}`.
    #[error("the file name must end with .{{ext}} so the files of an asset keep distinct names")]
    MissingExtension,
    /// The template starts with `/`.
    #[error("the first segment is empty; a name template must not start with /")]
    LeadingSlash,
    /// Two `/` follow each other, or the template ends with `/`.
    #[error("empty segment; a name template must not contain // or end with /")]
    EmptySegment,
    /// A segment is `.` or `..`.
    #[error("segment {segment:?} would leave the destination")]
    DotSegment {
        /// The segment, `.` or `..`.
        segment: String,
    },
}
