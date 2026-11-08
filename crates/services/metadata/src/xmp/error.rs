// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

use std::path::PathBuf;

use thiserror::Error;

/// Why an XMP sidecar could not be read (CORE-009.11–.14).
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("cannot read the XMP sidecar {}: {reason}", path.display())]
pub struct XmpError {
    /// The sidecar.
    pub path: PathBuf,
    /// What is wrong with it.
    pub reason: XmpErrorReason,
}

/// The reasons of an [`XmpError`].
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum XmpErrorReason {
    /// The content is not UTF-8, such as UTF-16.
    #[error("it is not UTF-8")]
    NotUtf8,
    /// The XML is not well-formed.
    #[error("the XML is malformed at line {line}, column {column}: {message}")]
    Malformed {
        /// The line of the error, from 1.
        line: usize,
        /// The column of the error in characters, from 1.
        column: usize,
        /// The tokenizer's description.
        message: String,
    },
    /// A document type declaration, which could declare entities.
    #[error("it contains a document type declaration, which XMP does not allow")]
    DocumentType,
    /// Well-formed XML that is not an XMP packet in RDF.
    #[error("it is not XMP: {0}")]
    NotXmp(String),
    /// One property is given two different values.
    #[error("the property {property} has two different values")]
    Conflict {
        /// The property as written in the file, such as `xmp:Rating`.
        property: String,
    },
}

/// Why a value could not be set (CORE-009, merging).
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum XmpEditError {
    /// The property exists with a value of another form.
    #[error("the property {property} is not {expected}")]
    WrongForm {
        /// The property's namespace URI and local name.
        property: String,
        /// The form the change needs, such as "an unordered array".
        expected: &'static str,
    },
}

/// The clock shows an instant outside the years 1 to 9999, which an XMP date cannot
/// hold.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("the clock shows a time outside the years 1 to 9999")]
pub struct ClockOutOfRange;
