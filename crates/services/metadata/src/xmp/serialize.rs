// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Writes a packet in Framehive's canonical form (ADR 0039, decisions 3 and 4).

use std::collections::BTreeMap;

use domain::{Clock, LocalDateTime, Timestamp, UtcOffset};

use super::error::ClockOutOfRange;
use super::model::{ArrayKind, XmpName, XmpNode, XmpPacket, XmpValue};
use super::namespaces::{ADOBE_META, RDF, XML, XMP_BASIC};
use super::prefixes;
use super::xml::{XmlAttribute, XmlEvent, XmlName, XmlStart, XmlSyntax};

const INDENT: &str = "  ";

/// Serializes `packet` in the canonical form, with `xmp:MetadataDate` set to the
/// clock's local time (CORE-009, writing).
///
/// `xmp:CreatorTool`, `xmp:ModifyDate` and `xmpMM:History` are never touched.
///
/// # Errors
///
/// [`ClockOutOfRange`] when the clock shows a year an XMP date cannot hold.
pub fn serialize_xmp(
    packet: &XmpPacket,
    clock: &dyn Clock,
    xml: &dyn XmlSyntax,
) -> Result<String, ClockOutOfRange> {
    let now = clock.now();
    let date = xmp_date(now, clock.local_offset(now)).ok_or(ClockOutOfRange)?;
    let mut properties = packet.properties.clone();
    properties.insert(
        XmpName::new(XMP_BASIC, "MetadataDate"),
        XmpNode::text(&date),
    );
    let prefixes = prefixes::choose(&properties, &packet.prefixes);
    let mut text = xml.write(&Writer { prefixes }.packet(&properties));
    text.push('\n');
    Ok(text)
}

/// An XMP date with seconds and offset, such as `2026-11-08T14:03:12+01:00`.
fn xmp_date(instant: Timestamp, offset: UtcOffset) -> Option<String> {
    let local = LocalDateTime::from_timestamp(instant, offset)?;
    let minutes = offset.seconds() / 60;
    let sign = if minutes < 0 { '-' } else { '+' };
    Some(format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}{sign}{:02}:{:02}",
        local.year(),
        local.month(),
        local.day(),
        local.hour(),
        local.minute(),
        local.second(),
        minutes.abs() / 60,
        minutes.abs() % 60,
    ))
}

struct Writer {
    /// The chosen prefix of every namespace the packet uses.
    prefixes: BTreeMap<String, String>,
}

impl Writer {
    fn packet(&self, properties: &BTreeMap<XmpName, XmpNode>) -> Vec<XmlEvent> {
        let mut events = vec![start(
            XmlName::new(ADOBE_META, "x", "xmpmeta"),
            Vec::new(),
            vec![("x".to_owned(), ADOBE_META.to_owned())],
        )];
        indent(&mut events, 1);
        events.push(start(
            rdf("RDF"),
            Vec::new(),
            vec![("rdf".to_owned(), RDF.to_owned())],
        ));
        indent(&mut events, 2);
        let mut declarations: Vec<_> = self
            .prefixes
            .iter()
            .map(|(namespace, prefix)| (prefix.clone(), namespace.clone()))
            .collect();
        declarations.sort();
        events.push(start(
            rdf("Description"),
            vec![attribute(rdf("about"), "")],
            declarations,
        ));
        self.fields(&mut events, properties, 3);
        events.push(XmlEvent::End);
        indent(&mut events, 1);
        events.push(XmlEvent::End);
        indent(&mut events, 0);
        events.push(XmlEvent::End);
        events
    }

    /// Writes fields ordered by prefix, then by name, each on its own line, and the
    /// line break before the parent's end tag.
    fn fields(
        &self,
        events: &mut Vec<XmlEvent>,
        fields: &BTreeMap<XmpName, XmpNode>,
        depth: usize,
    ) {
        let mut ordered: Vec<_> = fields
            .iter()
            .map(|(name, node)| (self.name(name), node))
            .collect();
        ordered.sort_by(|(left, _), (right, _)| {
            (&left.prefix, &left.local).cmp(&(&right.prefix, &right.local))
        });
        for (name, node) in &ordered {
            indent(events, depth);
            self.node(events, name.clone(), node, depth);
        }
        if !ordered.is_empty() {
            indent(events, depth - 1);
        }
    }

    fn node(&self, events: &mut Vec<XmlEvent>, name: XmlName, node: &XmpNode, depth: usize) {
        let mut attributes = Vec::new();
        let mut qualifiers = node.qualifiers.clone();
        if let Some(language) = node.language() {
            attributes.push(attribute(XmlName::new(XML, "xml", "lang"), language));
            qualifiers.remove(&XmpName::language());
        }
        if !qualifiers.is_empty() {
            attributes.push(attribute(rdf("parseType"), "Resource"));
            events.push(start(name, attributes, Vec::new()));
            indent(events, depth + 1);
            self.value(events, rdf("value"), Vec::new(), &node.value, depth + 1);
            self.fields(events, &qualifiers, depth + 1);
            events.push(XmlEvent::End);
            return;
        }
        self.value(events, name, attributes, &node.value, depth);
    }

    fn value(
        &self,
        events: &mut Vec<XmlEvent>,
        name: XmlName,
        mut attributes: Vec<XmlAttribute>,
        value: &XmpValue,
        depth: usize,
    ) {
        match value {
            XmpValue::Text(text) => {
                events.push(start(name, attributes, Vec::new()));
                events.push(XmlEvent::Text(text.clone()));
            }
            XmpValue::Uri(uri) => {
                attributes.push(attribute(rdf("resource"), uri));
                events.push(start(name, attributes, Vec::new()));
            }
            XmpValue::Struct(fields) => {
                attributes.push(attribute(rdf("parseType"), "Resource"));
                events.push(start(name, attributes, Vec::new()));
                self.fields(events, fields, depth + 1);
            }
            XmpValue::Array(kind, items) => {
                events.push(start(name, attributes, Vec::new()));
                indent(events, depth + 1);
                events.push(start(rdf(array_name(*kind)), Vec::new(), Vec::new()));
                for item in items {
                    indent(events, depth + 2);
                    self.node(events, rdf("li"), item, depth + 2);
                }
                if !items.is_empty() {
                    indent(events, depth + 1);
                }
                events.push(XmlEvent::End);
                indent(events, depth);
            }
        }
        events.push(XmlEvent::End);
    }

    fn name(&self, name: &XmpName) -> XmlName {
        let prefix = self
            .prefixes
            .get(&name.namespace)
            .map_or("", String::as_str);
        XmlName::new(&name.namespace, prefix, &name.local)
    }
}

fn array_name(kind: ArrayKind) -> &'static str {
    match kind {
        ArrayKind::Bag => "Bag",
        ArrayKind::Seq => "Seq",
        ArrayKind::Alt => "Alt",
    }
}

fn indent(events: &mut Vec<XmlEvent>, depth: usize) {
    events.push(XmlEvent::Text(format!("\n{}", INDENT.repeat(depth))));
}

fn start(
    name: XmlName,
    attributes: Vec<XmlAttribute>,
    namespaces: Vec<(String, String)>,
) -> XmlEvent {
    XmlEvent::Start(XmlStart {
        name,
        attributes,
        namespaces,
    })
}

fn attribute(name: XmlName, value: &str) -> XmlAttribute {
    XmlAttribute {
        name,
        value: value.to_owned(),
    }
}

fn rdf(local: &str) -> XmlName {
    XmlName::new(RDF, "rdf", local)
}
