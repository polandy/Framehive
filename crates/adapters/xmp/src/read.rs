// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Turns the tokens of `quick-xml` into the events of the port.

use metadata::xmp::{
    XmlAttribute, XmlEvent, XmlName, XmlStart, XmlSyntaxError, XmlSyntaxErrorKind,
};
use quick_xml::XmlVersion;
use quick_xml::escape::resolve_predefined_entity;
use quick_xml::events::{BytesRef, BytesStart, Event};
use quick_xml::name::QName;
use quick_xml::reader::Reader;

const XMLNS: &str = "xmlns";
const XML_PREFIX: &str = "xml";
const XML_NAMESPACE: &str = "http://www.w3.org/XML/1998/namespace";

/// The namespace declarations in effect, one list per open element.
#[derive(Default)]
struct Scopes(Vec<Vec<(String, String)>>);

impl Scopes {
    /// The namespace of `prefix`; `None` when it is not declared.
    fn namespace(&self, prefix: &str) -> Option<&str> {
        if prefix == XML_PREFIX {
            return Some(XML_NAMESPACE);
        }
        let declared = self
            .0
            .iter()
            .rev()
            .flatten()
            .find(|(declared, _)| declared == prefix)
            .map(|(_, namespace)| namespace.as_str());
        match declared {
            None if prefix.is_empty() => Some(""),
            Some("") if !prefix.is_empty() => None,
            other => other,
        }
    }
}

/// The events of `text`, or its first error.
pub(crate) fn events(text: &str) -> Result<Vec<XmlEvent>, XmlSyntaxError> {
    let mut reader = Reader::from_str(text);
    reader.config_mut().expand_empty_elements = true;
    let mut events = Vec::new();
    let mut scopes = Scopes::default();
    loop {
        let offset = offset(reader.buffer_position());
        let event = reader
            .read_event()
            .map_err(|error| malformed(error_offset(&reader), &error.to_string()))?;
        match event {
            Event::Start(start) => {
                let tag = start_tag(&mut scopes, &start, offset)?;
                events.push(XmlEvent::Start(tag));
            }
            Event::End(_) => {
                events.push(XmlEvent::End);
                scopes.0.pop();
            }
            Event::Text(text) => events.push(XmlEvent::Text(text.xml10_content().into_owned())),
            Event::CData(data) => events.push(XmlEvent::Text(data.xml10_content().into_owned())),
            Event::GeneralRef(reference) => {
                events.push(XmlEvent::Text(reference_text(&reference, offset)?));
            }
            Event::DocType(_) => {
                return Err(XmlSyntaxError {
                    offset,
                    kind: XmlSyntaxErrorKind::DocumentType,
                });
            }
            Event::Empty(_) | Event::Comment(_) | Event::Decl(_) | Event::PI(_) => {}
            Event::Eof => break,
        }
    }
    if !scopes.0.is_empty() {
        return Err(malformed(text.len(), "an element is not closed at the end"));
    }
    Ok(events)
}

/// The start tag with its names resolved; its declarations open a new scope.
fn start_tag(
    scopes: &mut Scopes,
    start: &BytesStart<'_>,
    offset: usize,
) -> Result<XmlStart, XmlSyntaxError> {
    let mut tag = XmlStart::default();
    let mut attributes = Vec::new();
    for attribute in start.attributes() {
        let attribute = attribute.map_err(|error| malformed(offset, &error.to_string()))?;
        if attribute.value.contains('<') {
            return Err(malformed(offset, "an attribute value contains '<'"));
        }
        let value = attribute
            .normalized_value(XmlVersion::Implicit1_0)
            .map_err(|error| malformed(offset, &error.to_string()))?
            .into_owned();
        let key = attribute.key;
        if key.as_ref() == XMLNS {
            tag.namespaces.push((String::new(), value));
        } else if key.prefix().is_some_and(|prefix| prefix.as_ref() == XMLNS) {
            let prefix: &str = key.local_name().into_inner();
            if !is_name(prefix) {
                return Err(malformed(offset, &format!("{prefix} is not an XML name")));
            }
            tag.namespaces.push((prefix.to_owned(), value));
        } else {
            attributes.push((key, value));
        }
    }
    scopes.0.push(tag.namespaces.clone());
    tag.name = resolved(scopes, start.name(), true, offset)?;
    for (key, value) in attributes {
        let name = resolved(scopes, key, false, offset)?;
        tag.attributes.push(XmlAttribute { name, value });
    }
    Ok(tag)
}

/// The name with its namespace; an unprefixed attribute is in no namespace.
fn resolved(
    scopes: &Scopes,
    name: QName<'_>,
    element: bool,
    offset: usize,
) -> Result<XmlName, XmlSyntaxError> {
    let prefix = name.prefix();
    let prefix = prefix.as_ref().map_or("", AsRef::as_ref);
    let local = name.local_name();
    let valid_prefix = prefix.is_empty() || is_name(prefix);
    if !valid_prefix || !is_name(local.as_ref()) {
        let qualified: &str = name.as_ref();
        return Err(malformed(
            offset,
            &format!("{qualified} is not an XML name"),
        ));
    }
    let namespace = if !element && prefix.is_empty() {
        Some("")
    } else {
        scopes.namespace(prefix)
    };
    let namespace = namespace
        .ok_or_else(|| malformed(offset, &format!("the prefix {prefix} is not declared")))?;
    Ok(XmlName::new(namespace, prefix, local.as_ref()))
}

/// An XML name without a colon (Namespaces in XML, `NCName`); characters beyond
/// ASCII are all accepted, which is wider than XML but never ambiguous.
fn is_name(name: &str) -> bool {
    let mut characters = name.chars();
    let starts_well = characters
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic() || first == '_' || !first.is_ascii());
    starts_well
        && characters.all(|next| {
            next.is_ascii_alphanumeric() || matches!(next, '_' | '-' | '.') || !next.is_ascii()
        })
}

/// A character reference or one of the five predefined entities; no other entity
/// exists without a document type declaration.
fn reference_text(reference: &BytesRef<'_>, offset: usize) -> Result<String, XmlSyntaxError> {
    if let Some(character) = reference
        .resolve_char_ref()
        .map_err(|error| malformed(offset, &error.to_string()))?
    {
        return Ok(character.to_string());
    }
    let name: &str = reference.as_ref();
    resolve_predefined_entity(name)
        .map(str::to_owned)
        .ok_or_else(|| malformed(offset, &format!("the entity &{name}; is not defined")))
}

fn error_offset(reader: &Reader<&[u8]>) -> usize {
    offset(reader.error_position())
}

fn offset(position: u64) -> usize {
    usize::try_from(position).unwrap_or(usize::MAX)
}

fn malformed(offset: usize, message: &str) -> XmlSyntaxError {
    XmlSyntaxError {
        offset,
        kind: XmlSyntaxErrorKind::Malformed(message.to_owned()),
    }
}
