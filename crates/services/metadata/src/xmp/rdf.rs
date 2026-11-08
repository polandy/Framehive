// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Reads the RDF of an XMP packet into the model, accepting every form ISO 16684-1
//! allows (ADR 0039, decision 1).

use std::collections::BTreeMap;

use super::error::XmpErrorReason;
use super::model::{ArrayKind, XmpName, XmpNode, XmpPacket, XmpValue};
use super::namespaces::{ADOBE_META, RDF, XML};
use super::tree::Element;
use super::xml::XmlName;

type Fields = BTreeMap<XmpName, XmpNode>;

/// The packet in the tree whose root is `x:xmpmeta` or `rdf:RDF`.
pub(crate) fn packet(root: &Element) -> Result<XmpPacket, XmpErrorReason> {
    let rdf = rdf_element(root)?;
    let mut reader = Reader::default();
    let mut properties = Fields::new();
    for description in &rdf.children {
        if !description.is(RDF, "Description") {
            return Err(unexpected(description, "in rdf:RDF"));
        }
        reader.description(description, &mut properties, true)?;
    }
    no_text(rdf)?;
    Ok(XmpPacket {
        properties,
        prefixes: reader.prefixes,
    })
}

fn rdf_element(root: &Element) -> Result<&Element, XmpErrorReason> {
    if root.is(RDF, "RDF") {
        return Ok(root);
    }
    let is_meta = root.name.namespace == ADOBE_META
        && (root.name.local == "xmpmeta" || root.name.local == "xapmeta");
    if !is_meta {
        return Err(not_xmp(format!(
            "the root element is {}, not x:xmpmeta or rdf:RDF",
            root.name.qualified()
        )));
    }
    match root.children.as_slice() {
        [rdf] if rdf.is(RDF, "RDF") && !root.has_text() => Ok(rdf),
        _ => Err(not_xmp(format!(
            "{} must hold exactly one rdf:RDF",
            root.name.qualified()
        ))),
    }
}

#[derive(Default)]
struct Reader {
    prefixes: BTreeMap<String, String>,
}

impl Reader {
    /// Adds the properties of an `rdf:Description` to `fields`: its attributes and
    /// its child elements. A nested one may hold `rdf:value`, the top level may not.
    fn description(
        &mut self,
        description: &Element,
        fields: &mut Fields,
        top_level: bool,
    ) -> Result<(), XmpErrorReason> {
        for attribute in &description.attributes {
            let name = &attribute.name;
            if is_ignored_rdf(name) || name.namespace == XML {
                continue;
            }
            if name.namespace == RDF && (top_level || name.local != "value") {
                return Err(not_xmp(format!(
                    "unexpected attribute {}",
                    name.qualified()
                )));
            }
            self.insert(fields, name, XmpNode::text(&attribute.value))?;
        }
        self.child_properties(description, fields, top_level)
    }

    /// Adds the child elements of a description or of an `rdf:parseType="Resource"`
    /// element to `fields`.
    fn child_properties(
        &mut self,
        description: &Element,
        fields: &mut Fields,
        top_level: bool,
    ) -> Result<(), XmpErrorReason> {
        for child in &description.children {
            if child.name.namespace == RDF && (top_level || child.name.local != "value") {
                return Err(unexpected(child, "as a property"));
            }
            let node = self.property(child)?;
            self.insert(fields, &child.name, node)?;
        }
        no_text(description)
    }

    /// The value of a property element, in whichever form it is written.
    fn property(&mut self, element: &Element) -> Result<XmpNode, XmpErrorReason> {
        let mut node = self.property_value(element)?;
        if let Some(language) = element.attribute(XML, "lang") {
            node.qualifiers
                .insert(XmpName::language(), XmpNode::text(language));
        }
        Ok(node)
    }

    fn property_value(&mut self, element: &Element) -> Result<XmpNode, XmpErrorReason> {
        let field_attributes: Vec<_> = element
            .attributes
            .iter()
            .filter(|attribute| {
                let name = &attribute.name;
                name.namespace != XML && (name.namespace != RDF || name.local == "value")
            })
            .collect();
        for attribute in &element.attributes {
            let known = ["parseType", "resource", "value", "ID", "nodeID"];
            if attribute.name.namespace == RDF && !known.contains(&attribute.name.local.as_str()) {
                return Err(not_xmp(format!(
                    "unexpected attribute {} on {}",
                    attribute.name.qualified(),
                    element.name.qualified()
                )));
            }
        }
        if let Some(parse_type) = element.attribute(RDF, "parseType") {
            if parse_type != "Resource" {
                return Err(not_xmp(format!(
                    "rdf:parseType=\"{parse_type}\" on {} is not XMP",
                    element.name.qualified()
                )));
            }
            if !field_attributes.is_empty() {
                return Err(unexpected_content(element));
            }
            let mut fields = Fields::new();
            self.child_properties(element, &mut fields, false)?;
            return Ok(resource(fields));
        }
        let mut fields = Fields::new();
        for attribute in &field_attributes {
            self.insert(
                &mut fields,
                &attribute.name,
                XmpNode::text(&attribute.value),
            )?;
        }
        if let Some(uri) = element.attribute(RDF, "resource") {
            if !element.children.is_empty() || element.has_text() {
                return Err(unexpected_content(element));
            }
            let mut node = XmpNode::new(XmpValue::Uri(uri.to_owned()));
            node.qualifiers = fields;
            return Ok(node);
        }
        if !fields.is_empty() {
            if !element.children.is_empty() || element.has_text() {
                return Err(unexpected_content(element));
            }
            return Ok(resource(fields));
        }
        match element.children.as_slice() {
            [] => Ok(XmpNode::text(&element.text)),
            [child] if !element.has_text() => self.nested(element, child),
            _ => Err(unexpected_content(element)),
        }
    }

    /// The value of a property whose only child is an array or an `rdf:Description`.
    fn nested(&mut self, parent: &Element, child: &Element) -> Result<XmpNode, XmpErrorReason> {
        if child.is(RDF, "Description") {
            let mut fields = Fields::new();
            self.description(child, &mut fields, false)?;
            return Ok(resource(fields));
        }
        let kind = match (child.name.namespace.as_str(), child.name.local.as_str()) {
            (RDF, "Bag") => ArrayKind::Bag,
            (RDF, "Seq") => ArrayKind::Seq,
            (RDF, "Alt") => ArrayKind::Alt,
            _ => {
                return Err(unexpected(
                    child,
                    &format!("in {}", parent.name.qualified()),
                ));
            }
        };
        let mut items = Vec::new();
        for item in &child.children {
            if !item.is(RDF, "li") {
                return Err(unexpected(item, &format!("in {}", child.name.qualified())));
            }
            items.push(self.property(item)?);
        }
        no_text(child)?;
        Ok(XmpNode::new(XmpValue::Array(kind, items)))
    }

    /// Adds a field, accepting a repeat with an equal value (CORE-009.14).
    fn insert(
        &mut self,
        fields: &mut Fields,
        name: &XmlName,
        node: XmpNode,
    ) -> Result<(), XmpErrorReason> {
        if name.namespace.is_empty() {
            return Err(not_xmp(format!("{} is in no namespace", name.qualified())));
        }
        if !name.prefix.is_empty() {
            self.prefixes
                .entry(name.namespace.clone())
                .or_insert_with(|| name.prefix.clone());
        }
        let key = XmpName::new(&name.namespace, &name.local);
        match fields.get(&key) {
            Some(existing) if *existing == node => Ok(()),
            Some(_) => Err(XmpErrorReason::Conflict {
                property: name.qualified(),
            }),
            None => {
                fields.insert(key, node);
                Ok(())
            }
        }
    }
}

/// A struct, or a value with qualifiers when the fields hold `rdf:value`.
fn resource(mut fields: Fields) -> XmpNode {
    let Some(value) = fields.remove(&XmpName::new(RDF, "value")) else {
        return XmpNode::new(XmpValue::Struct(fields));
    };
    let mut node = value;
    node.qualifiers.extend(fields);
    node
}

fn is_ignored_rdf(name: &XmlName) -> bool {
    name.namespace == RDF && matches!(name.local.as_str(), "about" | "ID" | "nodeID")
}

fn no_text(element: &Element) -> Result<(), XmpErrorReason> {
    if element.has_text() {
        return Err(unexpected_content(element));
    }
    Ok(())
}

fn unexpected(element: &Element, place: &str) -> XmpErrorReason {
    not_xmp(format!(
        "unexpected element {} {place}",
        element.name.qualified()
    ))
}

fn unexpected_content(element: &Element) -> XmpErrorReason {
    not_xmp(format!(
        "{} mixes forms or holds unexpected content",
        element.name.qualified()
    ))
}

fn not_xmp(message: String) -> XmpErrorReason {
    XmpErrorReason::NotXmp(message)
}
