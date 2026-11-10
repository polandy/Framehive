// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! A changed public API snapshot needs a commit that release-please lists in
//! `CHANGELOG.md` (architecture §7, ADR 0045 §3).
//!
//! The files are compared with the same base as `check-spec-trailers` (ADR 0010 §4): the
//! merge base with the base branch, or the parent of `HEAD` on `main`. The commits since
//! that base and the pull request title, when given, are the candidate messages.

use crate::specs::trailers::Range;
use crate::specs::trailers::git::{Environment, Repository, git, resolve_range};

use super::SNAPSHOT;

/// Conventional Commit types that release-please lists in the changelog by default.
const LISTED_TYPES: [&str; 4] = ["feat", "fix", "perf", "revert"];

/// What changed since the base.
struct Changes {
    files: Vec<String>,
    messages: Vec<String>,
}

/// The violations of the rule. Outside CI they are only warnings, because the commit
/// message may not be written yet, and so is a base that cannot be found.
pub(crate) fn check(root: &std::path::Path) -> Result<Vec<String>, String> {
    let repository = Repository::at(root);
    let environment = Environment::from_process();
    let found = changes_since_base(&repository, &environment).map(|changes| violations(&changes));
    match found {
        Ok(found) if environment.in_ci => Ok(found),
        Ok(found) => {
            for violation in found {
                eprintln!("warning: {violation}");
            }
            Ok(Vec::new())
        }
        Err(reason) if environment.in_ci => {
            Err(format!("cannot find the changes since the base: {reason}"))
        }
        Err(reason) => {
            eprintln!("warning: changelog rule of the public API not checked: {reason}");
            Ok(Vec::new())
        }
    }
}

fn changes_since_base(
    repository: &Repository,
    environment: &Environment,
) -> Result<Changes, String> {
    let base = base(repository, environment)?;
    let mut messages = commit_messages(repository, &base)?;
    messages.extend(environment.pull_request_title.clone());
    Ok(Changes {
        files: changed_files(repository, &base)?,
        messages,
    })
}

/// The merge base with the base branch, or the parent of `HEAD` on `main`.
fn base(repository: &Repository, environment: &Environment) -> Result<String, String> {
    match resolve_range(repository, environment)? {
        Range::SinceBase(base) => Ok(git(repository, &["merge-base", &base, "HEAD"])
            .map_err(|_| format!("{base} and HEAD have no common commit"))?
            .trim()
            .to_owned()),
        Range::Head => Ok("HEAD^".to_owned()),
        Range::Unavailable(reason) => Err(reason),
    }
}

/// The repository paths changed since `base`, including uncommitted and new files.
fn changed_files(repository: &Repository, base: &str) -> Result<Vec<String>, String> {
    let changed = git(repository, &["diff", "--name-only", base])?;
    let untracked = git(repository, &["ls-files", "--others", "--exclude-standard"])?;
    Ok(changed
        .lines()
        .chain(untracked.lines())
        .map(str::to_owned)
        .collect())
}

/// The full messages of the commits in `base..HEAD`.
fn commit_messages(repository: &Repository, base: &str) -> Result<Vec<String>, String> {
    let log = git(
        repository,
        &["log", "--format=%B%x00", &format!("{base}..HEAD")],
    )?;
    Ok(log
        .split('\0')
        .map(str::trim)
        .filter(|message| !message.is_empty())
        .map(str::to_owned)
        .collect())
}

/// Whether release-please lists the commit: a listed type, or a breaking change.
fn is_listed(message: &str) -> bool {
    let subject = message.lines().next().unwrap_or_default();
    let Some((prefix, _)) = subject.split_once(':') else {
        return false;
    };
    let breaking = prefix.ends_with('!')
        || message.lines().any(|line| {
            line.starts_with("BREAKING CHANGE:") || line.starts_with("BREAKING-CHANGE:")
        });
    let kind = prefix
        .trim_end_matches('!')
        .split('(')
        .next()
        .unwrap_or_default();
    breaking || LISTED_TYPES.contains(&kind)
}

/// One violation if a snapshot changed but no message is listed in the changelog.
fn violations(changes: &Changes) -> Vec<String> {
    let snapshots: Vec<&str> = changes
        .files
        .iter()
        .map(String::as_str)
        .filter(|path| path.rsplit('/').next() == Some(SNAPSHOT))
        .collect();
    if snapshots.is_empty() || changes.messages.iter().any(|message| is_listed(message)) {
        return Vec::new();
    }
    vec![format!(
        "the public API changed ({}) without a feat, fix, perf, revert or breaking commit \
         for the changelog (ADR 0045 §3)",
        snapshots.join(", ")
    )]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn changes(files: &[&str], messages: &[&str]) -> Changes {
        let owned = |list: &[&str]| list.iter().map(|item| (*item).to_owned()).collect();
        Changes {
            files: owned(files),
            messages: owned(messages),
        }
    }

    #[test]
    fn a_changed_snapshot_without_a_listed_commit_is_a_violation() {
        let found = violations(&changes(
            &[
                "crates/kernel/public-api.txt",
                "crates/kernel/src/lib.rs",
                "crates/services/jobs/public-api.txt",
            ],
            &["refactor(kernel): split the module", "docs: explain it"],
        ));
        assert_eq!(
            found,
            ["the public API changed (crates/kernel/public-api.txt, \
              crates/services/jobs/public-api.txt) without a feat, fix, perf, revert or \
              breaking commit for the changelog (ADR 0045 §3)"]
        );
    }

    #[test]
    fn a_changed_snapshot_with_a_listed_commit_passes() {
        let snapshot = ["crates/kernel/public-api.txt"];
        for message in [
            "feat(kernel): add a command",
            "fix: keep the order",
            "perf(jobs): skip a copy",
            "revert: undo the command",
            "refactor(kernel)!: rename the event",
            "refactor: rename it\n\nBREAKING CHANGE: the event has a new name",
        ] {
            assert!(
                violations(&changes(&snapshot, &["docs: first", message])).is_empty(),
                "{message}"
            );
        }
    }

    #[test]
    fn a_hand_written_changelog_entry_alone_is_not_enough() {
        let found = violations(&changes(
            &["crates/kernel/public-api.txt", "CHANGELOG.md"],
            &["chore: update the snapshot"],
        ));
        assert_eq!(found.len(), 1);
    }

    /// Runs git in `root` with a fixed identity and no user configuration.
    fn scratch_git(root: &std::path::Path, arguments: &[&str]) {
        let status = std::process::Command::new("git")
            .args(arguments)
            .current_dir(root)
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_AUTHOR_NAME", "Test")
            .env("GIT_AUTHOR_EMAIL", "test@example.invalid")
            .env("GIT_COMMITTER_NAME", "Test")
            .env("GIT_COMMITTER_EMAIL", "test@example.invalid")
            .status()
            .expect("run git");
        assert!(status.success(), "git {arguments:?}");
    }

    #[test]
    fn changes_hold_commits_edits_new_files_and_messages_since_the_merge_base() {
        let directory = std::env::temp_dir().join(format!(
            "framehive-xtask-{}-public-api-changelog",
            std::process::id()
        ));
        // A leftover from an aborted run would make the result depend on it.
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).unwrap();
        let write = |file: &str| std::fs::write(directory.join(file), file).unwrap();
        scratch_git(&directory, &["init", "--quiet", "--initial-branch=main"]);
        write("CHANGELOG.md");
        write("old.txt");
        scratch_git(&directory, &["add", "--all"]);
        scratch_git(&directory, &["commit", "--quiet", "--message", "base"]);
        scratch_git(&directory, &["switch", "--quiet", "--create", "topic"]);
        write("public-api.txt");
        scratch_git(&directory, &["add", "--all"]);
        scratch_git(
            &directory,
            &["commit", "--quiet", "--message", "feat: change"],
        );
        std::fs::write(directory.join("old.txt"), "edited").unwrap();
        write("new.txt");

        let mut repository = Repository::at(&directory);
        repository.git_variables = vec![("GIT_CONFIG_GLOBAL".to_owned(), "/dev/null".to_owned())];
        let mut found = changes_since_base(&repository, &Environment::default()).unwrap();
        found.files.sort();
        let _ = std::fs::remove_dir_all(&directory);
        assert_eq!(found.files, ["new.txt", "old.txt", "public-api.txt"]);
        assert_eq!(found.messages, ["feat: change"]);
    }

    #[test]
    fn changes_outside_the_snapshots_need_no_listed_commit() {
        let found = violations(&changes(
            &["crates/kernel/src/lib.rs", "docs/public-api.txt.md"],
            &["chore: tidy"],
        ));
        assert!(found.is_empty());
    }
}
