// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Chooses the prefix of every namespace a packet uses (ADR 0039, decision 3).

use std::collections::{BTreeMap, BTreeSet};

use super::model::{XmpName, XmpNode, XmpValue};
use super::namespaces::{PREFERRED_PREFIXES, STRUCTURE_PREFIXES, XML, preferred_prefix};

/// The prefix of an unknown namespace whose file gave it none.
const FALLBACK_PREFIX: &str = "ns";

/// A prefix for every namespace of `properties`: the preferred one for known
/// namespaces, the file's own (`file_prefixes`) for the rest, made unique by a number.
pub(crate) fn choose(
    properties: &BTreeMap<XmpName, XmpNode>,
    file_prefixes: &BTreeMap<String, String>,
) -> BTreeMap<String, String> {
    let mut namespaces = BTreeSet::new();
    collect_fields(properties, &mut namespaces);
    namespaces.remove(XML);
    let mut taken: BTreeSet<String> = STRUCTURE_PREFIXES
        .iter()
        .chain(PREFERRED_PREFIXES.iter().map(|(_, prefix)| prefix))
        .map(|prefix| (*prefix).to_owned())
        .collect();
    let mut chosen = BTreeMap::new();
    let mut unknown = Vec::new();
    for namespace in namespaces {
        if let Some(prefix) = preferred_prefix(&namespace) {
            chosen.insert(namespace, prefix.to_owned());
        } else {
            let wanted = file_prefixes
                .get(&namespace)
                .filter(|prefix| usable(prefix))
                .map_or(FALLBACK_PREFIX, String::as_str)
                .to_owned();
            unknown.push((wanted, namespace));
        }
    }
    unknown.sort();
    for (wanted, namespace) in unknown {
        let prefix = unique(&wanted, &taken);
        taken.insert(prefix.clone());
        chosen.insert(namespace, prefix);
    }
    chosen
}

/// Prefixes starting with `xml` are reserved by XML.
fn usable(prefix: &str) -> bool {
    !prefix.is_empty() && !prefix.to_ascii_lowercase().starts_with("xml")
}

fn unique(wanted: &str, taken: &BTreeSet<String>) -> String {
    let mut candidate = wanted.to_owned();
    let mut number = 0_usize;
    while taken.contains(&candidate) {
        number += 1;
        candidate = format!("{wanted}{number}");
    }
    candidate
}

fn collect_fields(fields: &BTreeMap<XmpName, XmpNode>, namespaces: &mut BTreeSet<String>) {
    for (name, node) in fields {
        namespaces.insert(name.namespace.clone());
        collect_node(node, namespaces);
    }
}

fn collect_node(node: &XmpNode, namespaces: &mut BTreeSet<String>) {
    collect_fields(&node.qualifiers, namespaces);
    match &node.value {
        XmpValue::Text(_) | XmpValue::Uri(_) => {}
        XmpValue::Struct(fields) => collect_fields(fields, namespaces),
        XmpValue::Array(_, items) => {
            for item in items {
                collect_node(item, namespaces);
            }
        }
    }
}
