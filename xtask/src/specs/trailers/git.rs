// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Reads the commits to check from Git and runs the rules of the parent module on them.

use std::path::{Path, PathBuf};
use std::process::Command;

use super::{Range, choose_range, squash_message, violations};
use change::{read_commit, read_squash};

mod change;
#[cfg(test)]
mod repository_tests;

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
    /// `PULL_REQUEST_TITLE`: the subject of the future squash commit.
    pub(crate) pull_request_title: Option<String>,
    /// `PULL_REQUEST_BODY`: the body of the future squash commit.
    pub(crate) pull_request_body: Option<String>,
}

impl Environment {
    /// Reads the variables of the running process.
    pub(crate) fn from_process() -> Self {
        Self {
            in_ci: std::env::var("CI").is_ok_and(|value| value == "true"),
            base_branch: std::env::var("GITHUB_BASE_REF").ok(),
            github_ref: std::env::var("GITHUB_REF").ok(),
            pull_request_title: std::env::var("PULL_REQUEST_TITLE").ok(),
            pull_request_body: std::env::var("PULL_REQUEST_BODY").ok(),
        }
    }

    /// A pull request run in CI, where the title must be passed.
    fn requires_pull_request_message(&self) -> bool {
        self.in_ci
            && self
                .base_branch
                .as_deref()
                .is_some_and(|base| !base.is_empty())
    }
}

/// Checks the commits chosen from the environment and, when a pull request title is
/// given, the future squash commit. In CI (`CI=true`) a base that cannot be resolved is an
/// error, so the check never passes silently (ADR 0010 §4).
pub(crate) fn run(repository: &Repository, environment: &Environment) -> Result<Outcome, String> {
    let squash = squash_message(
        environment.pull_request_title.as_deref(),
        environment.pull_request_body.as_deref(),
        environment.requires_pull_request_message(),
    )?;
    let range = resolve_range(repository, environment)?;
    let commits = match &range {
        Range::SinceBase(base) => commits_since(repository, base),
        Range::Head => head_commit(repository),
        Range::Unavailable(reason) => Err(reason.clone()),
    };
    match commits {
        Ok(commits) => {
            let mut found = check_all(repository, &commits)?;
            // The squash commit only exists for a branch, never for `main` itself.
            if let (Range::SinceBase(base), Some(message)) = (&range, squash) {
                let title = environment
                    .pull_request_title
                    .as_deref()
                    .unwrap_or_default();
                found.extend(violations(&read_squash(repository, base, title, message)?));
            }
            Ok(Outcome::Checked(found))
        }
        Err(reason) if environment.in_ci => {
            Err(format!("cannot choose the commits to check: {reason}"))
        }
        Err(reason) => Ok(Outcome::Skipped(reason)),
    }
}

pub(crate) fn resolve_range(
    repository: &Repository,
    environment: &Environment,
) -> Result<Range, String> {
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

/// Runs git and returns its stdout; a non-zero exit is an error.
pub(crate) fn git(repository: &Repository, arguments: &[&str]) -> Result<String, String> {
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
