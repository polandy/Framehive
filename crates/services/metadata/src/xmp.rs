// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! XMP packets: the data model of ISO 16684-1, the RDF parser and the canonical
//! serializer (CORE-009, ADR 0039). XML tokens come through the [`XmlSyntax`] port.

mod edit;
mod error;
mod model;
pub mod namespaces;
mod parse;
mod prefixes;
mod rdf;
mod serialize;
mod tree;
mod xml;

pub use error::{ClockOutOfRange, XmpEditError, XmpError, XmpErrorReason};
pub use model::{ArrayKind, XmpName, XmpNode, XmpPacket, XmpValue};
pub use parse::parse_xmp;
pub use serialize::serialize_xmp;
pub use xml::{
    XmlAttribute, XmlEvent, XmlName, XmlStart, XmlSyntax, XmlSyntaxError, XmlSyntaxErrorKind,
};
