// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Reads the bytes of a sidecar into a packet (CORE-009, reading).

use std::path::Path;

use super::error::{XmpError, XmpErrorReason};
use super::model::XmpPacket;
use super::rdf;
use super::tree;
use super::xml::{XmlSyntax, XmlSyntaxError, XmlSyntaxErrorKind};

const UTF8_BYTE_ORDER_MARK: &str = "\u{feff}";

/// Parses the content of the sidecar at `path` into a packet.
///
/// `path` only names the file in an error. A UTF-8 byte order mark is skipped.
///
/// # Errors
///
/// An [`XmpError`] naming `path` when the content is not UTF-8, not well-formed XML,
/// holds a document type declaration, is not an XMP packet, or gives one property two
/// different values.
pub fn parse_xmp(path: &Path, content: &[u8], xml: &dyn XmlSyntax) -> Result<XmpPacket, XmpError> {
    parse(content, xml).map_err(|reason| XmpError {
        path: path.to_path_buf(),
        reason,
    })
}

fn parse(content: &[u8], xml: &dyn XmlSyntax) -> Result<XmpPacket, XmpErrorReason> {
    let text = std::str::from_utf8(content).map_err(|_| XmpErrorReason::NotUtf8)?;
    // UTF-16 and UTF-32 text in ASCII range is valid UTF-8 with NUL bytes between.
    if text.contains('\0') {
        return Err(XmpErrorReason::NotUtf8);
    }
    let text = text.strip_prefix(UTF8_BYTE_ORDER_MARK).unwrap_or(text);
    let events = xml.read(text).map_err(|error| syntax_error(text, &error))?;
    let root = tree::root(events).map_err(|message| malformed(text, text.len(), message))?;
    rdf::packet(&root)
}

fn syntax_error(text: &str, error: &XmlSyntaxError) -> XmpErrorReason {
    match &error.kind {
        XmlSyntaxErrorKind::DocumentType => XmpErrorReason::DocumentType,
        XmlSyntaxErrorKind::Malformed(message) => malformed(text, error.offset, message),
    }
}

/// A malformed-XML reason at the line and column of byte `offset`.
fn malformed(text: &str, offset: usize, message: &str) -> XmpErrorReason {
    let mut end = offset.min(text.len());
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    let before = &text[..end];
    let line_start = before.rfind('\n').map_or(0, |at| at + 1);
    XmpErrorReason::Malformed {
        line: before.matches('\n').count() + 1,
        column: before[line_start..].chars().count() + 1,
        message: message.to_owned(),
    }
}
