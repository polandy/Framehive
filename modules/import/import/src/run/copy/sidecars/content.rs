// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The import's values as a sidecar packet, on their own or merged into a source
//! sidecar that came along (IMPORT-019, IMPORT-020, ADR 0040 §5).

use metadata::xmp::namespaces::{DUBLIN_CORE, PHOTOSHOP};
use metadata::xmp::{XmpEditError, XmpName, XmpNode, XmpPacket, XmpValue};

use crate::metadata_values::Keyword;
use crate::report::{ExistingValue, ExistingValueResult, SidecarField};
use crate::run::ImportSettings;

/// The values of `settings` for a file of `year`, merged into `packet`: keywords are
/// unioned, the job code replaces the old one, a creator or copyright already present
/// is kept. Returns each value that was there and differs from the import's.
pub(super) fn merge_values(
    settings: &ImportSettings<'_>,
    year: u16,
    packet: &mut XmpPacket,
) -> Result<Vec<ExistingValue>, XmpEditError> {
    let metadata = settings.metadata;
    let mut existing = Vec::new();
    if let Some(job_code) = settings.job_code {
        let name = XmpName::new(PHOTOSHOP, "TransmissionReference");
        let old = texts(packet.property(&name)).into_iter().next();
        note(
            &mut existing,
            SidecarField::JobCode,
            old,
            job_code.as_str(),
            true,
        );
        packet.set_text(&name, job_code.as_str())?;
    }
    if !metadata.creator.is_empty() {
        let name = XmpName::new(DUBLIN_CORE, "creator");
        let old = texts(packet.property(&name));
        if old.is_empty() {
            let creators: Vec<&str> = metadata.creator.iter().map(String::as_str).collect();
            packet.replace_seq(&name, &creators)?;
        } else {
            let import = metadata.creator.join("; ");
            note(
                &mut existing,
                SidecarField::Creator,
                Some(old.join("; ")),
                &import,
                false,
            );
        }
    }
    if let Some(rights) = metadata.copyright_of(i32::from(year)) {
        let name = XmpName::new(DUBLIN_CORE, "rights");
        match default_text(packet.property(&name)) {
            None => packet.set_language(&name, "x-default", &rights)?,
            old => note(&mut existing, SidecarField::Copyright, old, &rights, false),
        }
    }
    let keywords: Vec<&[String]> = metadata.keywords.iter().map(Keyword::levels).collect();
    if !keywords.is_empty() {
        packet.add_keywords(&keywords)?;
    }
    Ok(existing)
}

/// Notes the value `old` of `field` when it differs from the import's.
fn note(
    existing: &mut Vec<ExistingValue>,
    field: SidecarField,
    old: Option<String>,
    import: &str,
    replaced: bool,
) {
    let Some(old) = old.filter(|old| old != import) else {
        return;
    };
    existing.push(ExistingValue {
        field,
        existing: old,
        import: import.to_owned(),
        result: if replaced {
            ExistingValueResult::Replaced
        } else {
            ExistingValueResult::Kept
        },
    });
}

/// The texts of a simple property or of the items of an array.
fn texts(node: Option<&XmpNode>) -> Vec<String> {
    match node.map(|node| &node.value) {
        Some(XmpValue::Text(text)) => vec![text.clone()],
        Some(XmpValue::Array(_, items)) => items.iter().filter_map(text_of).collect(),
        _ => Vec::new(),
    }
}

/// The `x-default` text of a language alternative, else its first text.
fn default_text(node: Option<&XmpNode>) -> Option<String> {
    let Some(XmpValue::Array(_, items)) = node.map(|node| &node.value) else {
        return texts(node).into_iter().next();
    };
    items
        .iter()
        .find(|item| {
            item.language()
                .is_some_and(|language| language.eq_ignore_ascii_case("x-default"))
        })
        .and_then(text_of)
        .or_else(|| items.iter().find_map(text_of))
}

fn text_of(node: &XmpNode) -> Option<String> {
    match &node.value {
        XmpValue::Text(text) => Some(text.clone()),
        _ => None,
    }
}
