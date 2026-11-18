// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Builds an element tree from the events of the [`XmlSyntax`](super::XmlSyntax) port.

use super::xml::{MAX_XML_DEPTH, XmlAttribute, XmlEvent, XmlName};

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
                if open.len() == MAX_XML_DEPTH {
                    return Err("the elements are nested too deeply");
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

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, reason = "tests may panic on bad fixtures")]

    use super::*;
    use crate::xmp::XmlStart;

    fn nested(depth: usize) -> Vec<XmlEvent> {
        let start = XmlEvent::Start(XmlStart {
            name: XmlName::new("", "", "a"),
            ..XmlStart::default()
        });
        let mut events = vec![start; depth];
        events.extend(vec![XmlEvent::End; depth]);
        events
    }

    /// spec: CORE-009.25
    #[test]
    fn nesting_deeper_than_the_limit_is_refused_whatever_the_tokenizer() {
        assert!(root(nested(MAX_XML_DEPTH)).is_ok());
        assert_eq!(
            root(nested(MAX_XML_DEPTH + 1)).unwrap_err(),
            "the elements are nested too deeply"
        );
    }
}
