// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Writes the events of the port as XML text, escaped with `quick-xml`.

use std::fmt::Write as _;

use metadata::xmp::{XmlEvent, XmlStart};
use quick_xml::escape::{minimal_escape, partial_escape};

/// The text of `events`, written exactly in order.
pub(crate) fn text(events: &[XmlEvent]) -> String {
    let mut text = String::new();
    let mut open = Vec::new();
    for (index, event) in events.iter().enumerate() {
        match event {
            XmlEvent::Start(start) => {
                let name = start.name.qualified();
                text.push('<');
                text.push_str(&name);
                write_attributes(&mut text, start);
                if matches!(events.get(index + 1), Some(XmlEvent::End)) {
                    text.push_str("/>");
                    open.push(None);
                } else {
                    text.push('>');
                    open.push(Some(name));
                }
            }
            XmlEvent::End => {
                if let Some(Some(name)) = open.pop() {
                    let _ = write!(text, "</{name}>");
                }
            }
            XmlEvent::Text(content) => {
                text.push_str(&partial_escape(content.as_str()).replace('\r', "&#13;"));
            }
        }
    }
    text
}

fn write_attributes(text: &mut String, start: &XmlStart) {
    for attribute in &start.attributes {
        write_attribute(text, &attribute.name.qualified(), &attribute.value);
    }
    for (prefix, namespace) in &start.namespaces {
        let name = if prefix.is_empty() {
            "xmlns".to_owned()
        } else {
            format!("xmlns:{prefix}")
        };
        write_attribute(text, &name, namespace);
    }
}

/// Escapes what attribute-value normalization would change, so the value reads back
/// as written.
fn write_attribute(text: &mut String, name: &str, value: &str) {
    let escaped = minimal_escape(value)
        .replace('"', "&quot;")
        .replace('\t', "&#9;")
        .replace('\n', "&#10;")
        .replace('\r', "&#13;");
    let _ = write!(text, " {name}=\"{escaped}\"");
}
