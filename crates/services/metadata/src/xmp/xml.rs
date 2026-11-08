// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The port that turns XML text into events and back (ADR 0039, decision 1).

/// Reads and writes XML as a list of [`XmlEvent`]s; the XMP model, the RDF parser and
/// the serializer stay in this crate, so the tokenizer is replaceable.
///
/// `adapter-xmp` implements it; [`crate::testing::check_xml_syntax`] is its
/// conformance suite.
pub trait XmlSyntax {
    /// Reads `text` into balanced events, with every name resolved to its namespace.
    ///
    /// The XML declaration, comments and processing instructions are skipped, CDATA
    /// sections and references become text, and line ends are normalized to LF.
    /// Adjacent [`XmlEvent::Text`] events may follow each other.
    ///
    /// # Errors
    ///
    /// The first well-formedness error, an undeclared prefix, an unknown entity or a
    /// document type declaration, with its byte offset in `text`.
    fn read(&self, text: &str) -> Result<Vec<XmlEvent>, XmlSyntaxError>;

    /// Writes `events` exactly in order, adding no white space of its own.
    ///
    /// Names are written as `prefix:local`; their namespace is ignored. A start tag
    /// writes its attributes and then its namespace declarations. A start tag
    /// directly followed by its end tag is written as an empty element `<a/>`. Text
    /// and attribute values are escaped so that [`XmlSyntax::read`] gives them back.
    fn write(&self, events: &[XmlEvent]) -> String;
}

/// One event of an XML document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum XmlEvent {
    /// A start tag; an empty element `<a/>` is read as a start tag and its end tag.
    Start(XmlStart),
    /// The end tag of the innermost open element.
    End,
    /// Character data, unescaped.
    Text(String),
}

/// A start tag.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct XmlStart {
    /// The element's name.
    pub name: XmlName,
    /// The attributes in document order, without namespace declarations.
    pub attributes: Vec<XmlAttribute>,
    /// The namespace declarations of this tag as (prefix, URI); the prefix is empty
    /// for the default namespace.
    pub namespaces: Vec<(String, String)>,
}

/// A name in an XML document.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct XmlName {
    /// The namespace URI the name belongs to; empty for none.
    pub namespace: String,
    /// The prefix as written; empty for none.
    pub prefix: String,
    /// The local part of the name.
    pub local: String,
}

/// An attribute of a start tag.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XmlAttribute {
    /// The attribute's name.
    pub name: XmlName,
    /// The value, unescaped.
    pub value: String,
}

/// Why a text is not well-formed XML, and where.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XmlSyntaxError {
    /// The byte offset in the text at which the error was found.
    pub offset: usize,
    /// What was wrong.
    pub kind: XmlSyntaxErrorKind,
}

/// The kinds of [`XmlSyntaxError`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum XmlSyntaxErrorKind {
    /// The text is not well-formed; the tokenizer's description.
    Malformed(String),
    /// A document type declaration, which XMP does not allow (ISO 16684-1).
    DocumentType,
}

impl XmlName {
    /// Creates a name from its namespace, prefix and local part.
    #[must_use]
    pub fn new(namespace: &str, prefix: &str, local: &str) -> Self {
        XmlName {
            namespace: namespace.to_owned(),
            prefix: prefix.to_owned(),
            local: local.to_owned(),
        }
    }

    /// The name as written: `prefix:local`, or `local` without a prefix.
    #[must_use]
    pub fn qualified(&self) -> String {
        if self.prefix.is_empty() {
            self.local.clone()
        } else {
            format!("{}:{}", self.prefix, self.local)
        }
    }
}
