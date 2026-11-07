// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! A changed public API snapshot needs a `CHANGELOG.md` entry (architecture §7).
//!
//! The files are compared with the same base as `check-spec-trailers` (ADR 0010 §4): the
//! merge base with the base branch, or the parent of `HEAD` on `main`.

use crate::specs::trailers::Range;
use crate::specs::trailers::git::{Environment, Repository, git, resolve_range};

use super::SNAPSHOT;

const CHANGELOG: &str = "CHANGELOG.md";

/// The violations of the rule; outside CI, a base that cannot be found is only a warning.
pub(crate) fn check(root: &std::path::Path) -> Result<Vec<String>, String> {
    let repository = Repository::at(root);
    let environment = Environment::from_process();
    match changed_files(&repository, &environment) {
        Ok(files) => Ok(violations(&files)),
        Err(reason) if environment.in_ci => Err(format!(
            "cannot find the files changed since the base: {reason}"
        )),
        Err(reason) => {
            eprintln!("warning: changelog rule of the public API not checked: {reason}");
            Ok(Vec::new())
        }
    }
}

/// The repository paths changed since the base, including uncommitted and new files.
fn changed_files(
    repository: &Repository,
    environment: &Environment,
) -> Result<Vec<String>, String> {
    let base = match resolve_range(repository, environment)? {
        Range::SinceBase(base) => git(repository, &["merge-base", &base, "HEAD"])
            .map_err(|_| format!("{base} and HEAD have no common commit"))?
            .trim()
            .to_owned(),
        Range::Head => "HEAD^".to_owned(),
        Range::Unavailable(reason) => return Err(reason),
    };
    let changed = git(repository, &["diff", "--name-only", &base])?;
    let untracked = git(repository, &["ls-files", "--others", "--exclude-standard"])?;
    Ok(changed
        .lines()
        .chain(untracked.lines())
        .map(str::to_owned)
        .collect())
}

/// One violation if a snapshot changed but `CHANGELOG.md` did not.
fn violations(changed: &[String]) -> Vec<String> {
    let snapshots: Vec<&str> = changed
        .iter()
        .map(String::as_str)
        .filter(|path| path.rsplit('/').next() == Some(SNAPSHOT))
        .collect();
    if snapshots.is_empty() || changed.iter().any(|path| path == CHANGELOG) {
        return Vec::new();
    }
    vec![format!(
        "the public API changed ({}) without a {CHANGELOG} entry (architecture §7)",
        snapshots.join(", ")
    )]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn paths(list: &[&str]) -> Vec<String> {
        list.iter().map(|path| (*path).to_owned()).collect()
    }

    #[test]
    fn a_changed_snapshot_without_a_changelog_entry_is_a_violation() {
        let found = violations(&paths(&[
            "crates/kernel/public-api.txt",
            "crates/kernel/src/lib.rs",
            "crates/services/jobs/public-api.txt",
        ]));
        assert_eq!(
            found,
            ["the public API changed (crates/kernel/public-api.txt, \
              crates/services/jobs/public-api.txt) without a CHANGELOG.md entry \
              (architecture §7)"]
        );
    }

    #[test]
    fn a_changed_snapshot_with_a_changelog_entry_passes() {
        let changed = paths(&["crates/kernel/public-api.txt", "CHANGELOG.md"]);
        assert!(violations(&changed).is_empty());
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
    fn changed_files_holds_commits_edits_and_new_files_since_the_merge_base() {
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
        scratch_git(&directory, &["commit", "--quiet", "--message", "change"]);
        std::fs::write(directory.join("old.txt"), "edited").unwrap();
        write("new.txt");

        let mut repository = Repository::at(&directory);
        repository.git_variables = vec![("GIT_CONFIG_GLOBAL".to_owned(), "/dev/null".to_owned())];
        let mut files = changed_files(&repository, &Environment::default()).unwrap();
        files.sort();
        let _ = std::fs::remove_dir_all(&directory);
        assert_eq!(files, ["new.txt", "old.txt", "public-api.txt"]);
    }

    #[test]
    fn changes_outside_the_snapshots_need_no_changelog_entry() {
        let changed = paths(&["crates/kernel/src/lib.rs", "docs/public-api.txt.md"]);
        assert!(violations(&changed).is_empty());
    }
}
