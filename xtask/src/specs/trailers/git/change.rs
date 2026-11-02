// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Reads what the rules need about one change from Git: a single commit, or the future
//! squash commit of a pull request.

use std::collections::{BTreeMap, BTreeSet};

use super::super::{Commit, spec_id_of, squash_summary};
use super::{Repository, failure, git, run_git};
use crate::specs::document::status_field;
use crate::specs::markers::{TEST_MARKER, test_markers};

pub(super) fn read_commit(repository: &Repository, id: &str) -> Result<Commit, String> {
    let parent = git(
        repository,
        &["rev-parse", "--verify", "--quiet", &format!("{id}^")],
    )
    .ok();
    let parent = parent.as_deref().map(str::trim);
    let changed_files = changed_files(repository, &[id])?;
    Ok(Commit {
        summary: git(repository, &["log", "-1", "--format=%h %s", id])?
            .trim()
            .to_owned(),
        message: git(repository, &["log", "-1", "--format=%B", id])?,
        ..read_change(repository, parent, id, changed_files)?
    })
}

/// The future squash commit: every change since the merge base with `base`, under the
/// pull request title and description.
pub(super) fn read_squash(
    repository: &Repository,
    base: &str,
    title: &str,
    message: String,
) -> Result<Commit, String> {
    let merge_base = git(repository, &["merge-base", base, "HEAD"])?;
    let merge_base = merge_base.trim();
    let changed_files = changed_files(repository, &[merge_base, "HEAD"])?;
    Ok(Commit {
        summary: squash_summary(title),
        message,
        ..read_change(repository, Some(merge_base), "HEAD", changed_files)?
    })
}

/// What the rules need about the change from `before` to `after`, apart from the message.
fn read_change(
    repository: &Repository,
    before: Option<&str>,
    after: &str,
    changed_files: Vec<String>,
) -> Result<Commit, String> {
    let markers_after = markers_at(repository, after)?;
    let markers_before = match before {
        Some(before) => markers_at(repository, before)?,
        None => BTreeMap::new(),
    };
    Ok(Commit {
        referenced_specs: referenced_specs(&changed_files, &markers_before, &markers_after),
        spec_statuses: spec_statuses(repository, after, before, &changed_files),
        marked_files: invert(&markers_after),
        crate_directories: crate_directories(repository, after)?,
        changed_files,
        ..Commit::default()
    })
}

/// Files changed by one commit (`[id]`) or between two revisions (`[from, to]`).
/// `diff-tree` is plumbing, so no rename detection merges two paths into one.
fn changed_files(repository: &Repository, revisions: &[&str]) -> Result<Vec<String>, String> {
    let mut arguments = vec![
        "diff-tree",
        "--root",
        "--no-commit-id",
        "-r",
        "--name-only",
        "-z",
    ];
    arguments.extend_from_slice(revisions);
    let listing = git(repository, &arguments)?;
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
