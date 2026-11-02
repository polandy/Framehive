// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! `cargo xtask check-spec-trailers`: the diff check of ADR 0010 §4.
//!
//! A commit that changes tests linked to a specification changes that specification too,
//! or says why not with `Spec-Unchanged: <reason>` (Rule A). A commit that changes an
//! implemented specification changes related code too, or says why not with
//! `Spec-Only: <reason>` (Rule B). This file holds the rules as pure functions; `git`
//! reads the commits.

use std::collections::{BTreeMap, BTreeSet};

pub(crate) mod git;

const SPEC_UNCHANGED: &str = "Spec-Unchanged";
const SPEC_ONLY: &str = "Spec-Only";
const IMPLEMENTED: &str = "implemented";

/// What the rules need to know about one commit.
#[derive(Debug, Default)]
pub(crate) struct Commit {
    /// Short hash and subject, to name the commit in a violation.
    pub(crate) summary: String,
    pub(crate) message: String,
    pub(crate) changed_files: Vec<String>,
    /// For each changed `.rs` file: the spec IDs its markers name before or after the commit.
    pub(crate) referenced_specs: BTreeMap<String, BTreeSet<String>>,
    /// For each changed spec file: its status after the commit, or before it if deleted.
    pub(crate) spec_statuses: BTreeMap<String, String>,
    /// For each spec ID: the files that hold markers for it after the commit.
    pub(crate) marked_files: BTreeMap<String, BTreeSet<String>>,
    /// Directories that contain a crate manifest, such as `modules/import/import`.
    pub(crate) crate_directories: Vec<String>,
}

/// Which commits to check, decided from the environment.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Range {
    /// Every commit in `<base>..HEAD`, merge commits left out.
    SinceBase(String),
    /// Only `HEAD`, unless it is a merge commit.
    Head,
    /// Nothing can be checked, for the given reason.
    Unavailable(String),
}

/// Chooses the commits to check (ADR 0010 §4, amendment of 2026-11-02).
///
/// `base_branch` is `GITHUB_BASE_REF`, `github_ref` is `GITHUB_REF`, `current_branch` is
/// the checked-out branch and `has_main` says whether a local `main` exists.
pub(crate) fn choose_range(
    base_branch: Option<&str>,
    github_ref: Option<&str>,
    current_branch: Option<&str>,
    has_main: bool,
) -> Range {
    if let Some(base) = base_branch.filter(|base| !base.is_empty()) {
        return Range::SinceBase(format!("origin/{base}"));
    }
    if github_ref == Some("refs/heads/main") || current_branch == Some("main") {
        return Range::Head;
    }
    if has_main {
        Range::SinceBase("main".to_owned())
    } else {
        Range::Unavailable("there is no local branch `main` to compare with".to_owned())
    }
}

/// The message of the future squash commit of a pull request, if it is to be judged
/// (ADR 0010 §4, amendment of 2026-11-02).
///
/// The squash commit is the title, a blank line and the description. `required` is true
/// in a pull request run in CI, where a missing title is an error; elsewhere the composed
/// commit is judged only when a title is given. An empty description is allowed: it just
/// carries no trailers.
pub(crate) fn squash_message(
    title: Option<&str>,
    description: Option<&str>,
    required: bool,
) -> Result<Option<String>, String> {
    match title.map(str::trim).filter(|title| !title.is_empty()) {
        Some(title) => Ok(Some(format!(
            "{title}\n\n{}",
            description.unwrap_or_default()
        ))),
        None if required => Err(
            "PULL_REQUEST_TITLE is not set; the pull request workflow must pass the title \
             and description to judge the future squash commit (ADR 0010)"
                .to_owned(),
        ),
        None => Ok(None),
    }
}

/// Names the future squash commit in a violation, so the fix is obvious: the trailer
/// belongs in the pull request description, not in a commit.
pub(crate) fn squash_summary(title: &str) -> String {
    format!(
        "pull request \"{}\" (title and description as the future squash commit; \
         put the trailer in the description)",
        title.trim()
    )
}

/// Returns every rule the commit breaks, one message each.
pub(crate) fn violations(commit: &Commit) -> Vec<String> {
    let mut found = Vec::new();
    if !has_trailer(&commit.message, SPEC_UNCHANGED) {
        found.extend(tests_without_spec(commit));
    }
    if !has_trailer(&commit.message, SPEC_ONLY) {
        found.extend(spec_without_code(commit));
    }
    found
}

/// Whether a line `<name>: <reason>` with a non-empty reason is anywhere in the message.
/// A squash commit collects the messages of all its commits, so the position is free.
fn has_trailer(message: &str, name: &str) -> bool {
    message.lines().any(|line| {
        line.trim()
            .strip_prefix(name)
            .and_then(|rest| rest.strip_prefix(':'))
            .is_some_and(|reason| !reason.trim().is_empty())
    })
}

/// Rule A: tests linked to a spec changed, the spec did not.
fn tests_without_spec(commit: &Commit) -> Vec<String> {
    let changed_specs: BTreeSet<&str> = commit
        .changed_files
        .iter()
        .filter_map(|file| spec_id_of(file))
        .collect();
    let mut untouched: BTreeMap<&str, &str> = BTreeMap::new();
    for (file, spec_ids) in &commit.referenced_specs {
        for spec_id in spec_ids {
            if !changed_specs.contains(spec_id.as_str()) {
                untouched.entry(spec_id).or_insert(file);
            }
        }
    }
    untouched
        .into_iter()
        .map(|(spec_id, file)| {
            format!(
                "{}: changes {file}, which tests {spec_id}, but not the spec of {spec_id}; \
                 change the spec or add `{SPEC_UNCHANGED}: <reason>` (ADR 0010)",
                commit.summary
            )
        })
        .collect()
}

/// Rule B: an implemented spec changed, no related code did.
fn spec_without_code(commit: &Commit) -> Vec<String> {
    commit
        .spec_statuses
        .iter()
        .filter(|(_, status)| status.as_str() == IMPLEMENTED)
        .filter_map(|(file, _)| Some((file, spec_id_of(file)?)))
        .filter(|(_, spec_id)| !related_code_changed(commit, spec_id))
        .map(|(file, spec_id)| {
            format!(
                "{}: changes the implemented spec {file}, but no code or test of {spec_id}; \
                 change them or add `{SPEC_ONLY}: <reason>` (ADR 0010)",
                commit.summary
            )
        })
        .collect()
}

/// Related code is any `.rs` file in a crate that holds markers for the spec, or any
/// `.rs` file at all when no crate does.
fn related_code_changed(commit: &Commit, spec_id: &str) -> bool {
    let related_crates: BTreeSet<Option<&str>> = commit
        .marked_files
        .get(spec_id)
        .into_iter()
        .flatten()
        .map(|file| crate_of(file, &commit.crate_directories))
        .collect();
    commit
        .changed_files
        .iter()
        .filter(|file| is_rust_file(file))
        .any(|file| {
            related_crates.is_empty()
                || related_crates.contains(&crate_of(file, &commit.crate_directories))
        })
}

fn is_rust_file(file: &str) -> bool {
    std::path::Path::new(file)
        .extension()
        .is_some_and(|extension| extension == "rs")
}

/// The innermost crate directory that contains `file`.
fn crate_of<'a>(file: &str, crate_directories: &'a [String]) -> Option<&'a str> {
    crate_directories
        .iter()
        .filter(|directory| {
            file.strip_prefix(directory.as_str())
                .is_some_and(|rest| rest.starts_with('/'))
        })
        .max_by_key(|directory| directory.len())
        .map(String::as_str)
}

/// The spec ID of a spec file, from its name: `specs/import/IMPORT-003-x.md` → `IMPORT-003`.
pub(crate) fn spec_id_of(file: &str) -> Option<&str> {
    let name = file.strip_prefix("specs/")?.rsplit('/').next()?;
    let stem = name.strip_suffix(".md")?;
    let mut dashes = stem.match_indices('-').map(|(index, _)| index);
    let first = dashes.next()?;
    let end = dashes.next().unwrap_or(stem.len());
    let number = name.get(first + 1..end)?;
    let valid = first > 0 && number.len() == 3 && number.chars().all(|c| c.is_ascii_digit());
    valid.then(|| &name[..end])
}

#[cfg(test)]
mod tests {
    use super::*;

    const SPEC: &str = "specs/import/IMPORT-003-conflicts.md";
    const TEST: &str = "modules/import/import/tests/conflicts.rs";
    const CODE: &str = "modules/import/import/src/plan.rs";

    fn commit(changed: &[&str]) -> Commit {
        Commit {
            summary: "abc1234 fix: something".to_owned(),
            changed_files: changed.iter().map(|file| (*file).to_owned()).collect(),
            crate_directories: vec!["modules/import/import".to_owned(), "apps/cli".to_owned()],
            ..Commit::default()
        }
    }

    fn with_markers(mut commit: Commit) -> Commit {
        let specs = BTreeSet::from(["IMPORT-003".to_owned()]);
        if commit.changed_files.iter().any(|file| file == TEST) {
            commit.referenced_specs.insert(TEST.to_owned(), specs);
        }
        commit
            .marked_files
            .insert("IMPORT-003".to_owned(), BTreeSet::from([TEST.to_owned()]));
        commit
    }

    fn implemented(mut commit: Commit) -> Commit {
        commit
            .spec_statuses
            .insert(SPEC.to_owned(), IMPLEMENTED.to_owned());
        commit
    }

    #[test]
    fn rule_a_requires_the_spec_or_spec_unchanged() {
        let changed = with_markers(commit(&[TEST]));
        let found = violations(&changed);
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].contains("Spec-Unchanged"), "{found:?}");

        let mut explained = with_markers(commit(&[TEST]));
        explained.message = "refactor: x\n\nSpec-Unchanged: pure test refactor\n".to_owned();
        assert!(violations(&explained).is_empty());

        let with_spec = implemented(with_markers(commit(&[TEST, SPEC])));
        assert!(violations(&with_spec).is_empty());
    }

    #[test]
    fn rule_b_requires_related_code_or_spec_only() {
        let found = violations(&implemented(with_markers(commit(&[SPEC]))));
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].contains("Spec-Only"), "{found:?}");

        let unrelated = implemented(with_markers(commit(&[SPEC, "apps/cli/src/main.rs"])));
        assert_eq!(violations(&unrelated).len(), 1);

        let related = implemented(with_markers(commit(&[SPEC, CODE])));
        assert!(violations(&related).is_empty());

        let mut explained = implemented(with_markers(commit(&[SPEC])));
        explained.message = "docs: x\n\nSpec-Only: rewording\n".to_owned();
        assert!(violations(&explained).is_empty());
    }

    #[test]
    fn rule_b_accepts_any_rust_file_when_no_crate_holds_markers() {
        let changed = implemented(commit(&[SPEC, "apps/cli/src/main.rs"]));
        assert!(violations(&changed).is_empty());
        let docs_only = implemented(commit(&[SPEC, "README.md"]));
        assert_eq!(violations(&docs_only).len(), 1);
    }

    #[test]
    fn rule_b_ignores_specs_that_are_not_implemented() {
        let mut changed = with_markers(commit(&[SPEC]));
        changed
            .spec_statuses
            .insert(SPEC.to_owned(), "accepted".to_owned());
        assert!(violations(&changed).is_empty());
    }

    #[test]
    fn squash_message_joins_title_and_description() {
        assert_eq!(
            squash_message(Some("feat: x"), Some("Why.\n\nSpec-Only: wording"), true),
            Ok(Some("feat: x\n\nWhy.\n\nSpec-Only: wording".to_owned()))
        );
        assert_eq!(
            squash_message(Some("feat: x"), None, true),
            Ok(Some("feat: x\n\n".to_owned()))
        );
    }

    #[test]
    fn squash_message_without_title_is_an_error_only_when_required() {
        assert!(squash_message(None, Some("body"), true).is_err());
        assert!(squash_message(Some("  "), None, true).is_err());
        assert_eq!(squash_message(None, Some("body"), false), Ok(None));
    }

    #[test]
    fn squash_violation_points_to_the_description() {
        let mut squash = with_markers(commit(&[TEST]));
        squash.summary = squash_summary("feat: x");
        let found = violations(&squash);
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(
            found[0].starts_with("pull request \"feat: x\"") && found[0].contains("description"),
            "{found:?}"
        );
    }

    #[test]
    fn trailer_needs_a_reason_and_may_stand_anywhere() {
        assert!(has_trailer("a\nSpec-Only: why\n\nmore", SPEC_ONLY));
        assert!(!has_trailer("a\nSpec-Only:   \n", SPEC_ONLY));
        assert!(!has_trailer("a\nSpec-Onlyx: why\n", SPEC_ONLY));
    }

    #[test]
    fn spec_id_comes_from_the_file_name() {
        assert_eq!(spec_id_of(SPEC), Some("IMPORT-003"));
        assert_eq!(spec_id_of("specs/core/CORE-001.md"), Some("CORE-001"));
        assert_eq!(spec_id_of("specs/README.md"), None);
        assert_eq!(spec_id_of("docs/IMPORT-003-x.md"), None);
        assert_eq!(spec_id_of("specs/import/IMPORT-03-x.md"), None);
    }

    #[test]
    fn crate_is_the_innermost_manifest_directory() {
        let directories = [
            "modules/import".to_owned(),
            "modules/import/import".to_owned(),
        ];
        assert_eq!(crate_of(CODE, &directories), Some("modules/import/import"));
        assert_eq!(crate_of("modules/importer/x.rs", &directories), None);
    }

    #[test]
    fn range_follows_pull_request_main_and_local_runs() {
        assert_eq!(
            choose_range(Some("main"), Some("refs/pull/3/merge"), None, false),
            Range::SinceBase("origin/main".to_owned())
        );
        assert_eq!(
            choose_range(Some(""), Some("refs/heads/main"), None, false),
            Range::Head
        );
        assert_eq!(choose_range(None, None, Some("main"), true), Range::Head);
        assert_eq!(
            choose_range(None, None, Some("feat/x"), true),
            Range::SinceBase("main".to_owned())
        );
        assert!(matches!(
            choose_range(None, None, Some("feat/x"), false),
            Range::Unavailable(_)
        ));
    }
}
