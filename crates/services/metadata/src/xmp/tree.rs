// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Builds an element tree from the events of the [`XmlSyntax`](super::XmlSyntax) port.

use super::xml::{XmlAttribute, XmlEvent, XmlName};

/// An element with its attributes, child elements and text.
#[derive(Debug, Default)]
pub(crate) struct Element {
    pub(crate) name: XmlName,
    pub(crate) attributes: Vec<XmlAttribute>,
    pub(crate) children: Vec<Element>,
    /// All text directly inside the element, joined.
    pub(crate) text: String,
}

impl Element {
    pub(crate) fn is(&self, namespace: &str, local: &str) -> bool {
        self.name.namespace == namespace && self.name.local == local
    }

    pub(crate) fn attribute(&self, namespace: &str, local: &str) -> Option<&str> {
        self.attributes
            .iter()
            .find(|attribute| {
                attribute.name.namespace == namespace && attribute.name.local == local
            })
            .map(|attribute| attribute.value.as_str())
    }

    pub(crate) fn has_text(&self) -> bool {
        !self.text.trim_matches(is_xml_space).is_empty()
    }
}

/// The white space of XML: space, tab, line feed and carriage return.
pub(crate) fn is_xml_space(character: char) -> bool {
    matches!(character, ' ' | '\t' | '\n' | '\r')
}

/// The root element, or why the events do not form exactly one.
pub(crate) fn root(events: Vec<XmlEvent>) -> Result<Element, &'static str> {
    let mut open: Vec<Element> = Vec::new();
    let mut root = None;
    for event in events {
        match event {
            XmlEvent::Start(start) => {
                if open.is_empty() && root.is_some() {
                    return Err("the document has more than one root element");
                }
                open.push(Element {
                    name: start.name,
                    attributes: start.attributes,
                    ..Element::default()
                });
            }
            XmlEvent::End => {
                let element = open.pop().ok_or("an end tag has no start tag")?;
                match open.last_mut() {
                    Some(parent) => parent.children.push(element),
                    None => root = Some(element),
                }
            }
            XmlEvent::Text(text) => match open.last_mut() {
                Some(element) => element.text.push_str(&text),
                None if text.chars().all(is_xml_space) => {}
                None => return Err("the document has text outside its root element"),
            },
        }
    }
    if !open.is_empty() {
        return Err("an element is not closed");
    }
    root.ok_or("the document has no root element")
}
