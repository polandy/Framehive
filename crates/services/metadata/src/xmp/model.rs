// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The XMP data model of ISO 16684-1 (ADR 0039).

use std::collections::BTreeMap;
use std::fmt;

use super::namespaces::XML;

/// The name of a property, struct field or qualifier: its namespace URI and local name.
///
/// Prefixes carry no meaning in XMP (ISO 16684-1), so a name never holds one.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct XmpName {
    /// The namespace URI, such as `http://purl.org/dc/elements/1.1/`.
    pub namespace: String,
    /// The local name, such as `subject`.
    pub local: String,
}

/// A value with its qualifiers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XmpNode {
    /// The value.
    pub value: XmpValue,
    /// The qualifiers, `xml:lang` among them.
    pub qualifiers: BTreeMap<XmpName, XmpNode>,
}

/// The value of a property, struct field, array item or qualifier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum XmpValue {
    /// A simple value, the text exactly as read.
    Text(String),
    /// A simple value that is a URI (`rdf:resource`).
    Uri(String),
    /// A struct: named fields in no order.
    Struct(BTreeMap<XmpName, XmpNode>),
    /// An array and its items in order.
    Array(ArrayKind, Vec<XmpNode>),
}

/// The three kinds of XMP arrays.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArrayKind {
    /// An unordered array, `rdf:Bag`.
    Bag,
    /// An ordered array, `rdf:Seq`.
    Seq,
    /// Alternatives, `rdf:Alt`; with `xml:lang` on each item, a language alternative.
    Alt,
}

/// One XMP packet: top-level properties and the prefixes the file declared.
///
/// Two packets are equal when their properties are: prefixes carry no meaning
/// (ISO 16684-1), but the serializer keeps the file's prefix for unknown namespaces.
#[derive(Debug, Clone, Default)]
pub struct XmpPacket {
    pub(crate) properties: BTreeMap<XmpName, XmpNode>,
    /// The prefix the file used for each namespace URI.
    pub(crate) prefixes: BTreeMap<String, String>,
}

impl XmpName {
    /// Creates a name from a namespace URI and a local name.
    #[must_use]
    pub fn new(namespace: &str, local: &str) -> Self {
        XmpName {
            namespace: namespace.to_owned(),
            local: local.to_owned(),
        }
    }

    pub(crate) fn language() -> Self {
        XmpName::new(XML, "lang")
    }
}

impl fmt::Display for XmpName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}{}", self.namespace, self.local)
    }
}

impl XmpNode {
    /// A node holding `value` without qualifiers.
    #[must_use]
    pub fn new(value: XmpValue) -> Self {
        XmpNode {
            value,
            qualifiers: BTreeMap::new(),
        }
    }

    /// A simple text value without qualifiers.
    #[must_use]
    pub fn text(text: &str) -> Self {
        XmpNode::new(XmpValue::Text(text.to_owned()))
    }

    /// The `xml:lang` qualifier, when it is simple text.
    #[must_use]
    pub fn language(&self) -> Option<&str> {
        match &self.qualifiers.get(&XmpName::language())?.value {
            XmpValue::Text(language) => Some(language),
            _ => None,
        }
    }
}

impl XmpPacket {
    /// An empty packet.
    #[must_use]
    pub fn new() -> Self {
        XmpPacket::default()
    }

    /// The top-level properties.
    #[must_use]
    pub fn properties(&self) -> &BTreeMap<XmpName, XmpNode> {
        &self.properties
    }

    /// The top-level property `name`, if the packet has it.
    #[must_use]
    pub fn property(&self, name: &XmpName) -> Option<&XmpNode> {
        self.properties.get(name)
    }
}

impl PartialEq for XmpPacket {
    fn eq(&self, other: &Self) -> bool {
        self.properties == other.properties
    }
}

impl Eq for XmpPacket {}
