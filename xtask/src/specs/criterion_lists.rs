// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Header fields that list criterion numbers: `- Pending:` and `- E2e-owed:`.

use super::document::{NONE_MARKER, Spec, list_field};

/// Parses `- E2e-owed: 2, 4–6`, which only a `CLI-*` spec may have.
pub(super) fn e2e_owed_criteria(
    spec: &Spec,
    value: Option<&String>,
    problems: &mut Vec<String>,
) -> Vec<u32> {
    let Some(value) = value.filter(|value| value.as_str() != NONE_MARKER) else {
        return Vec::new();
    };
    if !spec.id.starts_with("CLI-") {
        let file = &spec.file;
        problems.push(format!("{file}: only a CLI spec has `- E2e-owed:`"));
        return Vec::new();
    }
    let numbers = criterion_numbers(spec, "e2e-owed", value, problems);
    for number in numbers
        .iter()
        .filter(|number| spec.pending.contains(number))
    {
        let (file, id) = (&spec.file, &spec.id);
        problems.push(format!(
            "{file}: e2e-owed criterion {id}.{number} is pending"
        ));
    }
    numbers
}

/// Parses a list of criterion numbers and ranges such as `1, 4–6`; `kind` names it in errors.
pub(super) fn criterion_numbers(
    spec: &Spec,
    kind: &str,
    value: &str,
    problems: &mut Vec<String>,
) -> Vec<u32> {
    let (file, id) = (&spec.file, &spec.id);
    let mut numbers = Vec::new();
    for entry in list_field(Some(value)) {
        let Some(range) = number_range(&entry) else {
            problems.push(format!("{file}: invalid {kind} criterion {entry:?}"));
            continue;
        };
        for number in range {
            if spec.removed.contains(&number) {
                problems.push(format!("{file}: {kind} criterion {id}.{number} is removed"));
            } else if !spec.criteria.contains(&number) {
                problems.push(format!(
                    "{file}: {kind} criterion {id}.{number} does not exist"
                ));
            } else {
                numbers.push(number);
            }
        }
    }
    numbers
}

/// `7` or `4–6`, with the en dash the origin ranges use.
fn number_range(entry: &str) -> Option<std::ops::RangeInclusive<u32>> {
    let (first, last) = entry.split_once(NONE_MARKER).unwrap_or((entry, entry));
    let (first, last) = (first.trim().parse().ok()?, last.trim().parse().ok()?);
    (first <= last).then_some(first..=last)
}

#[cfg(test)]
mod tests {
    use crate::specs::document::parse_spec;

    const VALID: &str = "# CORE-009 – Example\n\n- Status: implemented\n- Module: core\n\
                         - Origin: B19\n- Depends on: –\n\n\
                         ## Acceptance criteria\n\n### CORE-009.1 – One\n\n### CORE-009.2 – Two\n\n\
                         ## Open questions\n\n–\n\n## History\n";
    const CLI: &str = "specs/cli/CLI-009-example.md";

    #[test]
    fn a_cli_spec_lists_its_e2e_owed_criteria_with_ranges() {
        let text = VALID
            .replace("CORE-009", "CLI-009")
            .replace("implemented\n", "implemented\n- E2e-owed: 1–2\n");
        let spec = parse_spec(CLI, &text).expect("valid spec");
        assert_eq!(spec.e2e_owed, [1, 2]);
    }

    #[test]
    fn rejects_e2e_owed_criteria_outside_a_cli_spec() {
        let text = VALID.replace("implemented\n", "implemented\n- E2e-owed: 1\n");
        let problems = parse_spec("specs/core/CORE-009-example.md", &text).unwrap_err();
        assert_eq!(
            problems,
            ["specs/core/CORE-009-example.md: only a CLI spec has `- E2e-owed:`"]
        );
    }

    #[test]
    fn rejects_pending_unknown_and_malformed_e2e_owed_criteria() {
        let text = VALID.replace("CORE-009", "CLI-009").replace(
            "implemented\n",
            "partial\n- Pending: 2\n- E2e-owed: 2, 7, 3–1\n",
        );
        let problems = parse_spec(CLI, &text).unwrap_err();
        assert_eq!(
            problems,
            [
                "specs/cli/CLI-009-example.md: e2e-owed criterion CLI-009.7 does not exist",
                "specs/cli/CLI-009-example.md: invalid e2e-owed criterion \"3–1\"",
                "specs/cli/CLI-009-example.md: e2e-owed criterion CLI-009.2 is pending",
            ]
        );
    }
}
