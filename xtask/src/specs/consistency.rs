// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The cross-checks of `check-specs`: specs against each other, the requirement IDs and
//! the test markers (ADR 0010 §4).

use std::collections::{BTreeMap, BTreeSet};

use super::Report;
use super::document::{NONE_MARKER, Spec};
use super::levels::check_cli_system_tests;
use super::markers::Marker;

/// Checks parsed specs against each other, the requirement IDs and the test markers.
pub(crate) fn check(specs: &[Spec], requirements: &BTreeSet<String>, markers: &[Marker]) -> Report {
    let mut report = Report::default();
    let by_id = index_by_id(specs, &mut report);
    for spec in specs {
        check_origin(spec, requirements, &mut report);
        check_depends_on(spec, &by_id, &mut report);
        check_open_questions(spec, &mut report);
    }
    check_markers(&by_id, markers, &mut report);
    check_coverage(specs, markers, &mut report);
    check_cli_system_tests(specs, markers, &mut report);
    report
}

fn index_by_id<'a>(specs: &'a [Spec], report: &mut Report) -> BTreeMap<&'a str, &'a Spec> {
    let mut by_id = BTreeMap::new();
    for spec in specs {
        if by_id.insert(spec.id.as_str(), spec).is_some() {
            report
                .errors
                .push(format!("{}: duplicate spec ID {}", spec.file, spec.id));
        }
    }
    by_id
}

fn check_origin(spec: &Spec, requirements: &BTreeSet<String>, report: &mut Report) {
    for token in &spec.origin {
        if !origin_exists(token, requirements) {
            report.errors.push(format!(
                "{}: origin {token:?} is not in docs/dev/requirements/",
                spec.file
            ));
        }
    }
}

/// An origin is a requirement ID (`B19`), a range (`F1–F7`) or a section (`2.7`).
fn origin_exists(token: &str, requirements: &BTreeSet<String>) -> bool {
    match token.split_once(NONE_MARKER) {
        Some((first, last)) => requirements.contains(first) && requirements.contains(last),
        None => requirements.contains(token),
    }
}

fn check_depends_on(spec: &Spec, by_id: &BTreeMap<&str, &Spec>, report: &mut Report) {
    for dependency in &spec.depends_on {
        if by_id.contains_key(dependency.as_str()) {
            continue;
        }
        let message = format!("{}: depends on unknown spec {dependency}", spec.file);
        // Drafts may point to specs that are still to be written.
        if spec.status == "draft" {
            report.warnings.push(message);
        } else {
            report.errors.push(message);
        }
    }
}

fn check_open_questions(spec: &Spec, report: &mut Report) {
    let binding = ["accepted", "partial", "implemented"].contains(&spec.status.as_str());
    if binding && spec.has_open_questions {
        report.errors.push(format!(
            "{}: the {} spec must not have open questions",
            spec.file, spec.status
        ));
    }
}

fn check_markers(by_id: &BTreeMap<&str, &Spec>, markers: &[Marker], report: &mut Report) {
    for marker in markers {
        let Some(spec) = by_id.get(marker.spec_id.as_str()) else {
            report.errors.push(format!(
                "{}: unknown spec {}",
                marker.location,
                marker.criterion()
            ));
            continue;
        };
        if !spec.criteria.contains(&marker.number) {
            report.errors.push(format!(
                "{}: unknown criterion {}",
                marker.location,
                marker.criterion()
            ));
        } else if spec.removed.contains(&marker.number) {
            report.errors.push(format!(
                "{}: criterion {} is removed",
                marker.location,
                marker.criterion()
            ));
        } else if spec.pending.contains(&marker.number) {
            report.errors.push(format!(
                "{}: criterion {} is pending; drop it from `- Pending:`",
                marker.location,
                marker.criterion()
            ));
        } else if spec.status == "draft" || spec.status == "removed" {
            report.errors.push(format!(
                "{}: tests must not reference the {} spec {}",
                marker.location, spec.status, spec.id
            ));
        }
    }
}

fn check_coverage(specs: &[Spec], markers: &[Marker], report: &mut Report) {
    let reachable = |spec: &&Spec| spec.status == "implemented" || spec.status == "partial";
    for spec in specs.iter().filter(reachable) {
        let needs_test = |n: &&u32| !spec.removed.contains(n) && !spec.pending.contains(n);
        for number in spec.criteria.iter().filter(needs_test) {
            let tested = markers
                .iter()
                .any(|marker| marker.spec_id == spec.id && marker.number == *number);
            if !tested {
                report.errors.push(format!(
                    "{}: criterion {}.{number} has no test",
                    spec.file, spec.id
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::specs::document::parse_spec;
    use crate::specs::requirements::requirement_ids;

    const VALID: &str = "# CORE-009 – Example\n\n- Status: implemented\n- Module: core\n\
                         - Origin: B19, F1–F2, 2.7\n- Depends on: LIBRARY-001 (assets)\n\n\
                         ## Acceptance criteria\n\n### CORE-009.1 – One\n\n### CORE-009.2 – Two\n\n\
                         ## Open questions\n\n–\n\n## History\n";

    fn requirements() -> BTreeSet<String> {
        requirement_ids(
            "## 2. Import\n### 2.7 Formats\n| ID | Text |\n| B19 | x |\n| F1 | y |\n| F2 | z |\n",
        )
    }

    fn valid_spec() -> Spec {
        parse_spec("specs/core/CORE-009-example.md", VALID).expect("valid spec")
    }

    fn library_spec() -> Spec {
        Spec {
            id: "LIBRARY-001".to_owned(),
            status: "accepted".to_owned(),
            ..Spec::default()
        }
    }

    fn marker(spec_id: &str, number: u32) -> Marker {
        Marker {
            location: "x.rs:1".to_owned(),
            spec_id: spec_id.to_owned(),
            number,
        }
    }

    #[test]
    fn accepts_fully_tested_implemented_spec() {
        let specs = [valid_spec(), library_spec()];
        let report = check(
            &specs,
            &requirements(),
            &[marker("CORE-009", 1), marker("CORE-009", 2)],
        );
        assert!(report.errors.is_empty(), "{:?}", report.errors);
    }

    #[test]
    fn reports_untested_criterion_unknown_origin_and_marker() {
        let mut spec = valid_spec();
        spec.origin.push("Z9".to_owned());
        let report = check(
            &[spec, library_spec()],
            &requirements(),
            &[marker("CORE-009", 7)],
        );
        assert_eq!(report.errors.len(), 4, "{:?}", report.errors);
    }

    #[test]
    fn unknown_dependency_is_only_a_warning_for_drafts() {
        let mut spec = valid_spec();
        spec.status = "draft".to_owned();
        let report = check(&[spec], &requirements(), &[]);
        assert!(report.errors.is_empty(), "{:?}", report.errors);
        assert_eq!(report.warnings.len(), 1);
    }

    #[test]
    fn binding_spec_must_not_have_open_questions() {
        let text = VALID.replace("\n–\n", "\n- Is this clear?\n");
        let spec = parse_spec("specs/core/CORE-009-example.md", &text).expect("valid spec");
        let report = check(&[spec, library_spec()], &requirements(), &[]);
        assert!(report.errors.iter().any(|e| e.contains("open questions")));
    }

    #[test]
    fn tests_must_not_reference_drafts() {
        let mut spec = valid_spec();
        spec.status = "draft".to_owned();
        let report = check(
            &[spec, library_spec()],
            &requirements(),
            &[marker("CORE-009", 1)],
        );
        assert!(
            report.errors.iter().any(|e| e.contains("draft spec")),
            "{:?}",
            report.errors
        );
    }

    #[test]
    fn a_removed_criterion_needs_no_test_and_takes_none() {
        let mut spec = valid_spec();
        spec.removed.push(2);
        let specs = [spec, library_spec()];
        let report = check(&specs, &requirements(), &[marker("CORE-009", 1)]);
        assert!(report.errors.is_empty(), "{:?}", report.errors);
        let report = check(
            &specs,
            &requirements(),
            &[marker("CORE-009", 1), marker("CORE-009", 2)],
        );
        assert_eq!(report.errors, ["x.rs:1: criterion CORE-009.2 is removed"]);
    }

    fn partial_spec() -> Spec {
        let mut spec = valid_spec();
        spec.status = "partial".to_owned();
        spec.pending.push(2);
        spec
    }

    #[test]
    fn a_pending_criterion_needs_no_test_and_takes_none() {
        let specs = [partial_spec(), library_spec()];
        let report = check(&specs, &requirements(), &[marker("CORE-009", 1)]);
        assert!(report.errors.is_empty(), "{:?}", report.errors);
        let report = check(
            &specs,
            &requirements(),
            &[marker("CORE-009", 1), marker("CORE-009", 2)],
        );
        assert_eq!(
            report.errors,
            ["x.rs:1: criterion CORE-009.2 is pending; drop it from `- Pending:`"]
        );
    }

    #[test]
    fn every_other_criterion_of_a_partial_spec_needs_a_test() {
        let report = check(&[partial_spec(), library_spec()], &requirements(), &[]);
        assert_eq!(
            report.errors,
            ["specs/core/CORE-009-example.md: criterion CORE-009.1 has no test"]
        );
    }

    #[test]
    fn a_partial_spec_must_not_have_open_questions() {
        let mut spec = partial_spec();
        spec.has_open_questions = true;
        let report = check(
            &[spec, library_spec()],
            &requirements(),
            &[marker("CORE-009", 1)],
        );
        assert_eq!(
            report.errors,
            ["specs/core/CORE-009-example.md: the partial spec must not have open questions"]
        );
    }
}
