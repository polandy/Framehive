// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Parsing of one specification file (format in `specs/README.md`).

use std::collections::BTreeMap;

/// Prefixes from `specs/README.md`.
const PREFIXES: [&str; 9] = [
    "CORE",
    "IMPORT",
    "LIBRARY",
    "MAP",
    "PEOPLE",
    "SIMILARITY",
    "MIGRATE",
    "IMMICH",
    "CLI",
];
const STATUSES: [&str; 4] = ["draft", "accepted", "implemented", "removed"];
pub(super) const NONE_MARKER: &str = "–";

/// A parsed specification file.
#[derive(Debug, Default)]
pub(crate) struct Spec {
    pub(crate) file: String,
    pub(crate) id: String,
    pub(crate) status: String,
    pub(crate) origin: Vec<String>,
    pub(crate) depends_on: Vec<String>,
    /// Criterion numbers in document order, e.g. `[1, 2, 3]`.
    pub(crate) criteria: Vec<u32>,
    pub(crate) has_open_questions: bool,
}

/// Parses one spec file; returns every format problem at once.
pub(crate) fn parse_spec(file: &str, text: &str) -> Result<Spec, Vec<String>> {
    let mut spec = Spec {
        file: file.to_owned(),
        ..Spec::default()
    };
    let mut problems = Vec::new();
    match title_id(text) {
        Some(id) => id.clone_into(&mut spec.id),
        None => problems.push(format!("{file}: first line must be `# <ID> – <Title>`")),
    }
    if !spec.id.is_empty() {
        check_id(file, &spec.id, &mut problems);
    }
    let fields = header_fields(text);
    for key in ["Status", "Module", "Origin", "Depends on"] {
        if !fields.contains_key(key) {
            problems.push(format!("{file}: missing field `- {key}:`"));
        }
    }
    spec.status = fields.get("Status").cloned().unwrap_or_default();
    if !spec.status.is_empty() && !STATUSES.contains(&spec.status.as_str()) {
        problems.push(format!("{file}: unknown status {:?}", spec.status));
    }
    spec.origin = list_field(fields.get("Origin"));
    spec.depends_on = list_field(fields.get("Depends on"))
        .iter()
        .filter_map(|entry| entry.split_whitespace().next().map(str::to_owned))
        .collect();
    spec.criteria = criteria(file, &spec.id, text, &mut problems);
    spec.has_open_questions = has_open_questions(text);
    if problems.is_empty() {
        Ok(spec)
    } else {
        Err(problems)
    }
}

fn title_id(text: &str) -> Option<&str> {
    let title = text.lines().next()?.strip_prefix("# ")?;
    let (id, rest) = title.split_once(" – ")?;
    (!rest.trim().is_empty()).then_some(id)
}

fn check_id(file: &str, id: &str, problems: &mut Vec<String>) {
    let valid = id.split_once('-').is_some_and(|(prefix, number)| {
        PREFIXES.contains(&prefix)
            && number.len() == 3
            && number.chars().all(|c| c.is_ascii_digit())
    });
    if !valid {
        problems.push(format!("{file}: invalid spec ID {id:?}"));
    }
    let file_name = file.rsplit('/').next().unwrap_or(file);
    if !file_name.starts_with(&format!("{id}-")) {
        problems.push(format!("{file}: file name must start with \"{id}-\""));
    }
}

fn header_fields(text: &str) -> BTreeMap<String, String> {
    text.lines()
        .skip(1)
        .take_while(|line| !line.starts_with("## "))
        .filter_map(|line| line.strip_prefix("- ")?.split_once(": "))
        .map(|(key, value)| (key.to_owned(), value.trim().to_owned()))
        .collect()
}

/// Splits `A, B (note), C` at top-level commas; `–` means an empty list.
fn list_field(value: Option<&String>) -> Vec<String> {
    let Some(value) = value.filter(|value| value.as_str() != NONE_MARKER) else {
        return Vec::new();
    };
    let mut entries = Vec::new();
    let mut depth = 0_i32;
    let mut current = String::new();
    for character in value.chars() {
        match character {
            '(' => depth += 1,
            ')' => depth -= 1,
            ',' if depth == 0 => {
                entries.push(current.trim().to_owned());
                current.clear();
                continue;
            }
            _ => {}
        }
        current.push(character);
    }
    entries.push(current.trim().to_owned());
    entries.retain(|entry| !entry.is_empty());
    entries
}

fn criteria(file: &str, id: &str, text: &str, problems: &mut Vec<String>) -> Vec<u32> {
    let mut numbers = Vec::new();
    for heading in text.lines().filter_map(|line| line.strip_prefix("### ")) {
        let label = heading.split(" – ").next().unwrap_or(heading);
        let Some(number) = label.strip_prefix(&format!("{id}.")) else {
            continue;
        };
        match number.parse::<u32>() {
            Ok(number) if numbers.contains(&number) => {
                problems.push(format!("{file}: duplicate criterion {id}.{number}"));
            }
            Ok(number) => numbers.push(number),
            Err(_) => problems.push(format!("{file}: invalid criterion heading {heading:?}")),
        }
    }
    numbers
}

fn has_open_questions(text: &str) -> bool {
    let Some((_, rest)) = text.split_once("\n## Open questions\n") else {
        return false;
    };
    let section = rest.split("\n## ").next().unwrap_or(rest);
    section
        .lines()
        .map(str::trim)
        .any(|line| !line.is_empty() && line != NONE_MARKER)
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID: &str = "# CORE-009 – Example\n\n- Status: implemented\n- Module: core\n\
                         - Origin: B19, F1–F2, 2.7\n- Depends on: LIBRARY-001 (assets)\n\n\
                         ## Acceptance criteria\n\n### CORE-009.1 – One\n\n### CORE-009.2 – Two\n\n\
                         ## Open questions\n\n–\n\n## History\n";

    #[test]
    fn parses_a_valid_spec() {
        let spec = parse_spec("specs/core/CORE-009-example.md", VALID).expect("valid spec");
        assert_eq!(spec.id, "CORE-009");
        assert_eq!(spec.origin, ["B19", "F1–F2", "2.7"]);
        assert_eq!(spec.depends_on, ["LIBRARY-001"]);
        assert_eq!(spec.criteria, [1, 2]);
        assert!(!spec.has_open_questions);
    }

    #[test]
    fn rejects_malformed_header_and_file_name() {
        let problems = parse_spec("specs/core/CORE-01-x.md", "# CORE-01 Example\n").unwrap_err();
        assert!(
            problems.iter().any(|p| p.contains("first line")),
            "{problems:?}"
        );
        assert!(
            problems
                .iter()
                .any(|p| p.contains("missing field `- Status:`"))
        );
    }

    #[test]
    fn rejects_duplicate_criteria() {
        let text = VALID.replace("CORE-009.2", "CORE-009.1");
        let problems = parse_spec("specs/core/CORE-009-example.md", &text).unwrap_err();
        assert_eq!(
            problems,
            ["specs/core/CORE-009-example.md: duplicate criterion CORE-009.1"]
        );
    }
}
