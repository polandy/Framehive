// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The target names that conflict resolution tries and compares (IMPORT-003, ADR 0026).

use unicode_normalization::UnicodeNormalization;

/// The planned target, then `<base>-1.<ext>`, `<base>-2.<ext>`, …
pub(super) fn candidates(target: &str) -> impl Iterator<Item = String> + '_ {
    let (folder, name) = split_target(target);
    let (base, extension) = match name.rsplit_once('.') {
        Some((base, extension)) => (base, format!(".{extension}")),
        None => (name, String::new()),
    };
    let suffixed =
        (1_u64..).map(move |number| join_target(folder, &format!("{base}-{number}{extension}")));
    std::iter::once(target.to_owned()).chain(suffixed)
}

/// The folder part and the name of a target; the folder is empty at the top.
pub(super) fn split_target(target: &str) -> (&str, &str) {
    target.rsplit_once('/').unwrap_or(("", target))
}

pub(super) fn join_target(folder: &str, name: &str) -> String {
    if folder.is_empty() {
        name.to_owned()
    } else {
        format!("{folder}/{name}")
    }
}

/// The canonical caseless key of a name, `NFD(lowercase(NFD(name)))` (Unicode §3.13).
///
/// Lowercasing does not preserve a normalization form, so the standard's definition
/// decomposes the name both before and after it.
pub(crate) fn caseless_key(name: &str) -> String {
    let decomposed: String = name.nfd().collect();
    decomposed.to_lowercase().nfd().collect()
}
