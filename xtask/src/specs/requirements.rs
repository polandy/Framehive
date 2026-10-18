// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Requirement IDs and section numbers from `docs/requirements/`.

use std::collections::BTreeSet;

/// Requirement IDs (first table column, e.g. `B19`) and section numbers (e.g. `2.7`).
pub(crate) fn requirement_ids(requirements: &str) -> BTreeSet<String> {
    let mut ids = BTreeSet::new();
    for line in requirements.lines() {
        if let Some(cell) = line
            .strip_prefix('|')
            .and_then(|rest| rest.split('|').next())
        {
            let cell = cell.trim();
            if is_requirement_id(cell) {
                ids.insert(cell.to_owned());
            }
        }
        let heading = line.trim_start_matches('#');
        if heading.len() < line.len()
            && let Some(number) = heading.split_whitespace().next()
        {
            let number = number.trim_end_matches('.');
            if !number.is_empty() && number.chars().all(|c| c.is_ascii_digit() || c == '.') {
                ids.insert(number.to_owned());
            }
        }
    }
    ids
}

fn is_requirement_id(cell: &str) -> bool {
    let letters = cell.chars().take_while(char::is_ascii_uppercase).count();
    letters > 0 && cell.len() > letters && cell[letters..].chars().all(|c| c.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_requirement_ids_and_sections() {
        let text = "## 2. Import\n### 2.7 Formats\n| ID | Text |\n| B19 | x |\n| F1 | y |\n";
        let ids: Vec<_> = requirement_ids(text).into_iter().collect();
        assert_eq!(ids, ["2", "2.7", "B19", "F1"]);
    }
}
