// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Reads the commits to check from Git and runs the rules of the parent module on them.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;

use super::{Commit, Range, choose_range, spec_id_of, violations};
use crate::specs::document::status_field;
use crate::specs::markers::{TEST_MARKER, test_markers};

/// The outcome of a run: the violations, or why nothing was checked.
pub(crate) enum Outcome {
    Checked(Vec<String>),
    Skipped(String),
}

/// The Git checkout to read from.
pub(crate) struct Repository {
    pub(crate) root: PathBuf,
    /// Extra environment for every git call. Empty in production; tests use it to keep
    /// the developer's own Git configuration out of a throwaway repository.
    pub(crate) git_variables: Vec<(String, String)>,
}

impl Repository {
    /// The checkout at `root`, read with the caller's own Git configuration.
    pub(crate) fn at(root: &Path) -> Self {
        Self {
            root: root.to_path_buf(),
            git_variables: Vec::new(),
        }
    }
}

/// The parts of the process environment that choose what to check.
#[derive(Debug, Default)]
pub(crate) struct Environment {
    /// `CI=true`: a base that cannot be resolved is an error, not a notice.
    pub(crate) in_ci: bool,
    /// `GITHUB_BASE_REF`, set in a pull request run.
    pub(crate) base_branch: Option<String>,
    /// `GITHUB_REF`, such as `refs/heads/main`.
    pub(crate) github_ref: Option<String>,
}

impl Environment {
    /// Reads the variables of the running process.
    pub(crate) fn from_process() -> Self {
        Self {
            in_ci: std::env::var("CI").is_ok_and(|value| value == "true"),
            base_branch: std::env::var("GITHUB_BASE_REF").ok(),
            github_ref: std::env::var("GITHUB_REF").ok(),
        }
    }
}

/// Checks the commits chosen from the environment. In CI (`CI=true`) a base that cannot
/// be resolved is an error, so the check never passes silently (ADR 0010 §4).
pub(crate) fn run(repository: &Repository, environment: &Environment) -> Result<Outcome, String> {
    let range = resolve_range(repository, environment)?;
    let commits = match range {
        Range::SinceBase(base) => commits_since(repository, &base),
        Range::Head => head_commit(repository),
        Range::Unavailable(reason) => Err(reason),
    };
    match commits {
        Ok(commits) => check_all(repository, &commits).map(Outcome::Checked),
        Err(reason) if environment.in_ci => {
            Err(format!("cannot choose the commits to check: {reason}"))
        }
        Err(reason) => Ok(Outcome::Skipped(reason)),
    }
}

fn resolve_range(repository: &Repository, environment: &Environment) -> Result<Range, String> {
    let current_branch = git(repository, &["branch", "--show-current"])?;
    let has_main = git(
        repository,
        &["rev-parse", "--verify", "--quiet", "refs/heads/main"],
    )
    .is_ok();
    Ok(choose_range(
        environment.base_branch.as_deref(),
        environment.github_ref.as_deref(),
        Some(current_branch.trim()),
        has_main,
    ))
}

/// Non-merge commits in `<base>..HEAD`, oldest first.
fn commits_since(repository: &Repository, base: &str) -> Result<Vec<String>, String> {
    require_full_history(repository)?;
    git(
        repository,
        &[
            "rev-parse",
            "--verify",
            "--quiet",
            &format!("{base}^{{commit}}"),
        ],
    )
    .map_err(|_| format!("the base {base} does not exist"))?;
    git(repository, &["merge-base", base, "HEAD"])
        .map_err(|_| format!("{base} and HEAD have no common commit"))?;
    let listing = git(
        repository,
        &[
            "rev-list",
            "--reverse",
            "--no-merges",
            &format!("{base}..HEAD"),
        ],
    )?;
    Ok(listing.lines().map(str::to_owned).collect())
}

/// `HEAD` alone, or nothing if it is a merge commit.
fn head_commit(repository: &Repository) -> Result<Vec<String>, String> {
    require_full_history(repository)?;
    let listing = git(
        repository,
        &["rev-list", "--no-walk", "--no-merges", "HEAD"],
    )?;
    Ok(listing.lines().map(str::to_owned).collect())
}

/// In a shallow clone the oldest commit looks as if it added every file.
fn require_full_history(repository: &Repository) -> Result<(), String> {
    let shallow = git(repository, &["rev-parse", "--is-shallow-repository"])?;
    if shallow.trim() == "true" {
        return Err("the clone is shallow; fetch the full history".to_owned());
    }
    Ok(())
}

fn check_all(repository: &Repository, commits: &[String]) -> Result<Vec<String>, String> {
    let mut found = Vec::new();
    for id in commits {
        found.extend(violations(&read_commit(repository, id)?));
    }
    Ok(found)
}

fn read_commit(repository: &Repository, id: &str) -> Result<Commit, String> {
    let parent = git(
        repository,
        &["rev-parse", "--verify", "--quiet", &format!("{id}^")],
    )
    .ok();
    let parent = parent.as_deref().map(str::trim);
    let changed_files = changed_files(repository, id)?;
    let after = markers_at(repository, id)?;
    let before = match parent {
        Some(parent) => markers_at(repository, parent)?,
        None => BTreeMap::new(),
    };
    Ok(Commit {
        summary: git(repository, &["log", "-1", "--format=%h %s", id])?
            .trim()
            .to_owned(),
        message: git(repository, &["log", "-1", "--format=%B", id])?,
        referenced_specs: referenced_specs(&changed_files, &before, &after),
        spec_statuses: spec_statuses(repository, id, parent, &changed_files),
        marked_files: invert(&after),
        crate_directories: crate_directories(repository, id)?,
        changed_files,
    })
}

fn changed_files(repository: &Repository, id: &str) -> Result<Vec<String>, String> {
    let listing = git(
        repository,
        &[
            "diff-tree",
            "--root",
            "--no-commit-id",
            "-r",
            "--name-only",
            "-z",
            id,
        ],
    )?;
    Ok(listing
        .split('\0')
        .filter(|file| !file.is_empty())
        .map(str::to_owned)
        .collect())
}

/// Spec IDs named by markers, per `.rs` file, in the tree of `revision`.
fn markers_at(
    repository: &Repository,
    revision: &str,
) -> Result<BTreeMap<String, BTreeSet<String>>, String> {
    let arguments = [
        "grep",
        "-I",
        "-F",
        "-e",
        TEST_MARKER,
        revision,
        "--",
        "*.rs",
    ];
    // `git grep` exits with 1 when nothing matches.
    let output = run_git(repository, &arguments)?;
    match output.status.code() {
        Some(0 | 1) => Ok(parse_grep(
            revision,
            &String::from_utf8_lossy(&output.stdout),
        )),
        _ => Err(failure(&arguments, &output)),
    }
}

/// Parses `git grep <revision>` lines of the form `<revision>:<path>:<line>`.
fn parse_grep(revision: &str, listing: &str) -> BTreeMap<String, BTreeSet<String>> {
    let mut by_file: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let prefix = format!("{revision}:");
    for line in listing.lines() {
        let Some((file, text)) = line
            .strip_prefix(&prefix)
            .and_then(|rest| rest.split_once(':'))
        else {
            continue;
        };
        for marker in test_markers(file, text) {
            by_file
                .entry(file.to_owned())
                .or_default()
                .insert(marker.spec_id);
        }
    }
    by_file
}

fn referenced_specs(
    changed_files: &[String],
    before: &BTreeMap<String, BTreeSet<String>>,
    after: &BTreeMap<String, BTreeSet<String>>,
) -> BTreeMap<String, BTreeSet<String>> {
    let mut referenced = BTreeMap::new();
    for file in changed_files {
        let specs: BTreeSet<String> = [before.get(file), after.get(file)]
            .into_iter()
            .flatten()
            .flatten()
            .cloned()
            .collect();
        if !specs.is_empty() {
            referenced.insert(file.clone(), specs);
        }
    }
    referenced
}

fn invert(by_file: &BTreeMap<String, BTreeSet<String>>) -> BTreeMap<String, BTreeSet<String>> {
    let mut by_spec: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for (file, specs) in by_file {
        for spec in specs {
            by_spec
                .entry(spec.clone())
                .or_default()
                .insert(file.clone());
        }
    }
    by_spec
}

/// The status of each changed spec file after the commit, or before it if it was deleted.
fn spec_statuses(
    repository: &Repository,
    id: &str,
    parent: Option<&str>,
    changed_files: &[String],
) -> BTreeMap<String, String> {
    let mut statuses = BTreeMap::new();
    for file in changed_files
        .iter()
        .filter(|file| spec_id_of(file).is_some())
    {
        let text = [Some(id), parent]
            .into_iter()
            .flatten()
            .find_map(|revision| git(repository, &["show", &format!("{revision}:{file}")]).ok());
        if let Some(status) = text.as_deref().and_then(status_field) {
            statuses.insert(file.clone(), status);
        }
    }
    statuses
}

fn crate_directories(repository: &Repository, id: &str) -> Result<Vec<String>, String> {
    let listing = git(repository, &["ls-tree", "-r", "--name-only", id])?;
    Ok(listing
        .lines()
        .filter_map(|file| file.strip_suffix("/Cargo.toml"))
        .map(str::to_owned)
        .collect())
}

/// Runs git and returns its stdout; a non-zero exit is an error.
fn git(repository: &Repository, arguments: &[&str]) -> Result<String, String> {
    let output = run_git(repository, arguments)?;
    if !output.status.success() {
        return Err(failure(arguments, &output));
    }
    String::from_utf8(output.stdout).map_err(|error| error.to_string())
}

fn run_git(repository: &Repository, arguments: &[&str]) -> Result<std::process::Output, String> {
    Command::new("git")
        .args(arguments)
        .current_dir(&repository.root)
        .envs(
            repository
                .git_variables
                .iter()
                .map(|(name, value)| (name, value)),
        )
        .output()
        .map_err(|error| format!("cannot run git: {error}"))
}

fn failure(arguments: &[&str], output: &std::process::Output) -> String {
    format!(
        "git {} failed ({}):\n{}",
        arguments.join(" "),
        output.status,
        String::from_utf8_lossy(&output.stderr).trim_end()
    )
}

#[cfg(test)]
mod repository_tests;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grep_lines_become_spec_ids_per_file() {
        let listing = "abc:a/x.rs:    /// spec: CORE-001.1, CORE-002.3\nabc:b.rs:/// spec: CORE-001.2\n\
                       other:c.rs:/// spec: CORE-009.1\n";
        let by_file = parse_grep("abc", listing);
        assert_eq!(
            by_file.get("a/x.rs"),
            Some(&BTreeSet::from([
                "CORE-001".to_owned(),
                "CORE-002".to_owned()
            ]))
        );
        assert_eq!(by_file.len(), 2);
        let by_spec = invert(&by_file);
        assert_eq!(by_spec.get("CORE-001").map(BTreeSet::len), Some(2));
    }

    #[test]
    fn referenced_specs_join_before_and_after() {
        let before = BTreeMap::from([("a.rs".to_owned(), BTreeSet::from(["X-001".to_owned()]))]);
        let after = BTreeMap::from([("a.rs".to_owned(), BTreeSet::from(["X-002".to_owned()]))]);
        let changed = ["a.rs".to_owned(), "b.rs".to_owned()];
        let referenced = referenced_specs(&changed, &before, &after);
        assert_eq!(referenced.len(), 1);
        assert_eq!(referenced.get("a.rs").map(BTreeSet::len), Some(2));
    }
}
