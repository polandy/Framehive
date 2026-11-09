// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! XMP packets: the data model of ISO 16684-1, the RDF parser and the canonical
//! serializer (CORE-009, ADR 0039), and sidecar files read and saved through the
//! `files` port (ADR 0040). XML tokens come through the [`XmlSyntax`] port.

mod edit;
mod error;
mod keep_raw;
mod keywords;
mod model;
pub mod namespaces;
mod parse;
mod prefixes;
mod rdf;
mod save;
mod serialize;
mod sidecar;
mod tree;
mod xml;

pub use error::{
    ClockOutOfRange, SidecarError, SidecarErrorKind, SidecarStep, XmpEditError, XmpError,
    XmpErrorReason,
};
pub use model::{ArrayKind, XmpName, XmpNode, XmpPacket, XmpValue};
pub use parse::parse_xmp;
pub use save::{FoundSidecar, ReadSidecar, SidecarFiles};
pub use serialize::serialize_xmp;
pub use sidecar::{SidecarNaming, find_sidecars};
pub use xml::{
    XmlAttribute, XmlEvent, XmlName, XmlStart, XmlSyntax, XmlSyntaxError, XmlSyntaxErrorKind,
};
