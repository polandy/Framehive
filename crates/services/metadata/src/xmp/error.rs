// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

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

/// Why a sidecar file could not be read or saved (CORE-009.15–.19).
#[derive(Debug, Error)]
#[error("the XMP sidecar {} {kind}", path.display())]
pub struct SidecarError {
    /// The sidecar.
    pub path: PathBuf,
    /// What went wrong.
    pub kind: SidecarErrorKind,
    /// The `.part` file that could not be removed after the failure, if any.
    pub part_left: Option<PathBuf>,
}

impl SidecarError {
    pub(crate) fn new(path: &Path, kind: SidecarErrorKind) -> Self {
        SidecarError {
            path: path.to_path_buf(),
            kind,
            part_left: None,
        }
    }
}

/// The kinds of a [`SidecarError`].
#[derive(Debug, Error)]
pub enum SidecarErrorKind {
    /// The sidecar cannot be parsed; it is kept as it is.
    #[error("cannot be read: {0}")]
    Unreadable(XmpErrorReason),
    /// Something other than a file is at the path.
    #[error("is not a file")]
    NotAFile,
    /// Another program created a sidecar at the path during the write; it is kept.
    #[error("was created by another program meanwhile and is kept")]
    Appeared,
    /// Another program changed the sidecar during the write; the change is kept.
    #[error("was changed by another program meanwhile and is kept")]
    Changed,
    /// A change does not fit the property's form.
    #[error("cannot be changed: {0}")]
    Edit(XmpEditError),
    /// The clock cannot give `xmp:MetadataDate`.
    #[error("cannot be written: {0}")]
    Clock(ClockOutOfRange),
    /// A step of the file system failed.
    #[error("cannot be saved: {step} failed: {source}")]
    Io {
        /// The step that failed.
        step: SidecarStep,
        /// The file system's error.
        source: io::Error,
    },
}

/// The steps of reading and saving a sidecar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidecarStep {
    /// Reading the sidecar.
    Read,
    /// Writing the `.part` file.
    WritePart,
    /// Checking that the sidecar is as it was read.
    Check,
    /// Renaming the `.part` file to the sidecar.
    Rename,
    /// Syncing the sidecar's folder.
    SyncFolder,
}

impl fmt::Display for SidecarStep {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            SidecarStep::Read => "reading",
            SidecarStep::WritePart => "writing the .part file",
            SidecarStep::Check => "checking for changes",
            SidecarStep::Rename => "renaming the .part file",
            SidecarStep::SyncFolder => "syncing the folder",
        })
    }
}
