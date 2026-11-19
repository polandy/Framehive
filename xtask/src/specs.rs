// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! `cargo xtask check-specs`: specifications are well-formed, consistent with
//! `docs/dev/requirements/`, and linked to tests via `/// spec: <ID>.<n>` (ADR 0010).

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

mod consistency;
mod document;
mod markers;
mod report;
mod requirements;
pub(crate) mod trailers;

use consistency::check;
use document::{Spec, parse_spec};
use markers::{Marker, test_markers, unbound_markers};
use requirements::requirement_ids;

/// The requirements catalogue, one file per section.
const REQUIREMENTS_DIRECTORY: &str = "docs/dev/requirements";
/// Where `cargo xtask spec-report` writes; CI keeps it as an artifact.
const REPORT_FILE: &str = "target/spec-report.md";
/// Directories whose `.rs` files may contain test markers.
const SOURCE_DIRECTORIES: [&str; 4] = ["crates", "modules", "apps", "xtask"];

/// The outcome: errors fail the check, warnings are only printed.
#[derive(Debug, Default)]
pub(crate) struct Report {
    pub(crate) errors: Vec<String>,
    pub(crate) warnings: Vec<String>,
}

/// Everything the spec checks and the report read from the repository.
struct Repository {
    requirements: BTreeSet<String>,
    specs: Vec<Spec>,
    markers: Vec<Marker>,
    /// Markers that are not on a test function, one message each.
    unbound: Vec<String>,
    /// Spec files that could not be parsed, one message per problem.
    problems: Vec<String>,
}

/// Reads all specs, the requirements and the test markers below `root` and checks them.
pub(crate) fn check_repository(root: &Path) -> Result<Report, String> {
    let repository = load(root)?;
    let mut report = Report {
        errors: repository.problems,
        warnings: Vec::new(),
    };
    let checked = check(
        &repository.specs,
        &repository.requirements,
        &repository.markers,
    );
    report.errors.extend(checked.errors);
    report.errors.extend(repository.unbound);
    report.warnings.extend(checked.warnings);
    Ok(report)
}

/// Writes the overview requirement → spec → criteria → tests to `target/spec-report.md`.
pub(crate) fn write_report(root: &Path) -> Result<PathBuf, String> {
    let repository = load(root)?;
    if !repository.problems.is_empty() {
        return Err(format!(
            "cannot report on invalid specs:\n{}",
            repository.problems.join("\n")
        ));
    }
    let path = root.join(REPORT_FILE);
    let text = report::render(&repository.specs, &repository.markers);
    fs::write(&path, text).map_err(|error| format!("cannot write {}: {error}", path.display()))?;
    Ok(path)
}

fn load(root: &Path) -> Result<Repository, String> {
    let mut requirements = String::new();
    for path in files_with_extension(&root.join(REQUIREMENTS_DIRECTORY), "md")? {
        requirements.push_str(&read(&path)?);
    }
    let mut specs = Vec::new();
    let mut problems = Vec::new();
    for path in files_with_extension(&root.join("specs"), "md")? {
        if path.file_name().is_some_and(|name| name == "README.md") {
            continue;
        }
        let file = relative(root, &path);
        match parse_spec(&file, &read(&path)?) {
            Ok(spec) => specs.push(spec),
            Err(found) => problems.extend(found),
        }
    }
    let mut markers = Vec::new();
    let mut unbound = Vec::new();
    for directory in SOURCE_DIRECTORIES {
        for path in files_with_extension(&root.join(directory), "rs")? {
            let (file, source) = (relative(root, &path), read(&path)?);
            markers.extend(test_markers(&file, &source));
            unbound.extend(unbound_markers(&file, &source).into_iter().map(|location| {
                format!("{location}: a spec marker must sit on a #[test] function")
            }));
        }
    }
    Ok(Repository {
        requirements: requirement_ids(&requirements),
        specs,
        markers,
        unbound,
        problems,
    })
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
