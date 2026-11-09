// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The value of `{camera}` (IMPORT-021).

/// What `{camera}` shows when a file has no usable model.
const FALLBACK: &str = "Unknown camera";
/// Most characters of a model that are kept.
const MAX_CHARACTERS: usize = 64;
const INVALID_CHARACTERS: [char; 9] = ['/', '\\', '<', '>', ':', '"', '|', '?', '*'];

/// The text `{camera}` renders for `model`, valid in a file name in every mode.
pub(crate) fn name(model: Option<&str>) -> String {
    let model = model.unwrap_or_default();
    let model = model.split('\0').next().unwrap_or_default();
    let replaced: String = model
        .chars()
        .map(|c| {
            if c.is_control() && !c.is_whitespace() || INVALID_CHARACTERS.contains(&c) {
                '_'
            } else {
                c
            }
        })
        .collect();
    let single_blanks = replaced.split_whitespace().collect::<Vec<_>>().join(" ");
    let cut: String = trim_edges(&single_blanks)
        .chars()
        .take(MAX_CHARACTERS)
        .collect();
    match trim_edges(&cut) {
        "" => FALLBACK.to_owned(),
        text => text.to_owned(),
    }
}

fn trim_edges(text: &str) -> &str {
    text.trim_matches(|c: char| c == '.' || c.is_whitespace())
}
