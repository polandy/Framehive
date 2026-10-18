// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! `cargo xtask check-specs`: specifications are well-formed, consistent with
//! `docs/requirements/`, and linked to tests via `/// spec: <ID>.<n>` (ADR 0010).

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

mod document;
mod markers;
mod requirements;

use document::{NONE_MARKER, Spec, parse_spec};
use markers::{Marker, test_markers};
use requirements::requirement_ids;

/// Directories whose `.rs` files may contain test markers.
/// The requirements catalogue, one file per section.
const REQUIREMENTS_DIRECTORY: &str = "docs/requirements";
const SOURCE_DIRECTORIES: [&str; 3] = ["crates", "apps", "xtask"];

/// The outcome: errors fail the check, warnings are only printed.
#[derive(Debug, Default)]
pub(crate) struct Report {
    pub(crate) errors: Vec<String>,
    pub(crate) warnings: Vec<String>,
}

/// Reads all specs, the requirements and the test markers below `root` and checks them.
pub(crate) fn check_repository(root: &Path) -> Result<Report, String> {
    let mut requirements = String::new();
    for path in files_with_extension(&root.join(REQUIREMENTS_DIRECTORY), "md")? {
        requirements.push_str(&read(&path)?);
    }
    let mut specs = Vec::new();
    let mut report = Report::default();
    for path in files_with_extension(&root.join("specs"), "md")? {
        if path.file_name().is_some_and(|name| name == "README.md") {
            continue;
        }
        let file = relative(root, &path);
        match parse_spec(&file, &read(&path)?) {
            Ok(spec) => specs.push(spec),
            Err(problems) => report.errors.extend(problems),
        }
    }
    let mut markers = Vec::new();
    for directory in SOURCE_DIRECTORIES {
        for path in files_with_extension(&root.join(directory), "rs")? {
            markers.extend(test_markers(&relative(root, &path), &read(&path)?));
        }
    }
    let checked = check(&specs, &requirement_ids(&requirements), &markers);
    report.errors.extend(checked.errors);
    report.warnings.extend(checked.warnings);
    Ok(report)
}

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
                "{}: origin {token:?} is not in docs/requirements/",
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
    let binding = spec.status == "accepted" || spec.status == "implemented";
    if binding && spec.has_open_questions {
        report.errors.push(format!(
            "{}: an {} spec must not have open questions",
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
        } else if spec.status == "draft" || spec.status == "removed" {
            report.errors.push(format!(
                "{}: tests must not reference the {} spec {}",
                marker.location, spec.status, spec.id
            ));
        }
    }
}

fn check_coverage(specs: &[Spec], markers: &[Marker], report: &mut Report) {
    for spec in specs.iter().filter(|spec| spec.status == "implemented") {
        for number in &spec.criteria {
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

fn read(path: &Path) -> Result<String, String> {
    fs::read_to_string(path).map_err(|error| format!("cannot read {}: {error}", path.display()))
}

fn relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

/// All files with `extension` below `directory`, sorted; a missing directory is empty.
fn files_with_extension(directory: &Path, extension: &str) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();
    let mut pending = vec![directory.to_path_buf()];
    while let Some(current) = pending.pop() {
        let Ok(entries) = fs::read_dir(&current) else {
            continue;
        };
        for entry in entries {
            let path = entry.map_err(|error| error.to_string())?.path();
            if path.is_dir() && !path.ends_with("target") {
                pending.push(path);
            } else if path.extension().is_some_and(|found| found == extension) {
                files.push(path);
            }
        }
    }
    files.sort();
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
