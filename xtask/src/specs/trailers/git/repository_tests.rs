// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Runs the git wrapper against throwaway repositories, so the commit selection and the
//! reading of commits are tested without a real CI run.
//!
//! Every git call, the fixture's and the wrapper's, gets a fixed identity, fixed dates and
//! no global or system configuration, so the developer's own setup never leaks in.

use std::path::{Path, PathBuf};
use std::process::Command;

use super::{Environment, Outcome, Repository, commits_since, run};

const SPEC: &str = "specs/demo/DEMO-001-flow.md";
const SPEC_TEXT: &str = "# DEMO-001 – Flow\n\n- Status: implemented\n";
const TEST: &str = "demo/tests/flow.rs";
// The marker sits inside a string, so this file itself holds no marker.
const TEST_TEXT: &str = "/// spec: DEMO-001.1\n#[test]\nfn flows() {}\n";
const MANIFEST: &str = "demo/Cargo.toml";

/// A Git repository in a fresh directory under the system temp directory, removed on drop.
struct Scratch {
    directory: PathBuf,
    repository: Repository,
}

impl Scratch {
    /// An empty repository whose first branch is `branch`.
    fn new(name: &str, branch: &str) -> Self {
        let directory =
            std::env::temp_dir().join(format!("framehive-xtask-{}-{name}", std::process::id()));
        // A leftover from an aborted run would make the result depend on it.
        let _ = std::fs::remove_dir_all(&directory);
        let root = directory.join("repository");
        let home = directory.join("home");
        std::fs::create_dir_all(&root).expect("create the repository directory");
        std::fs::create_dir_all(&home).expect("create the home directory");
        let scratch = Self {
            repository: Repository {
                root,
                git_variables: isolated_git_variables(&home),
            },
            directory,
        };
        scratch.git(&["init", "--quiet", &format!("--initial-branch={branch}")]);
        scratch
    }

    /// Runs git in the repository and returns its stdout; a failure fails the test.
    fn git(&self, arguments: &[&str]) -> String {
        let output = Command::new("git")
            .args(arguments)
            .current_dir(&self.repository.root)
            .envs(
                self.repository
                    .git_variables
                    .iter()
                    .map(|(name, value)| (name, value)),
            )
            .output()
            .expect("run git");
        assert!(
            output.status.success(),
            "git {arguments:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).expect("utf-8 output")
    }

    fn write(&self, file: &str, text: &str) {
        let path = self.repository.root.join(file);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("create the parent directory");
        }
        std::fs::write(path, text).expect("write the file");
    }

    /// Commits every change and returns the new commit's full hash.
    fn commit(&self, message: &str) -> String {
        self.git(&["add", "--all"]);
        self.git(&["commit", "--quiet", "--message", message]);
        self.git(&["rev-parse", "HEAD"]).trim().to_owned()
    }

    /// The first commit: a crate with a test linked to an implemented spec.
    fn commit_linked_test_and_spec(&self) {
        self.write(MANIFEST, "[package]\nname = \"demo\"\n");
        self.write(TEST, TEST_TEXT);
        self.write(SPEC, SPEC_TEXT);
        self.commit("feat: add the flow");
    }

    /// A commit that changes the linked test alone, so Rule A fails without a trailer.
    fn commit_test_change(&self, message: &str) -> String {
        let text = std::fs::read_to_string(self.repository.root.join(TEST)).expect("read test");
        self.write(TEST, &format!("{text}// {message}\n"));
        self.commit(message)
    }

    fn run(&self, environment: &Environment) -> Result<Outcome, String> {
        run(&self.repository, environment)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.directory);
    }
}

fn isolated_git_variables(home: &Path) -> Vec<(String, String)> {
    let home = home.display().to_string();
    [
        ("GIT_CONFIG_GLOBAL", "/dev/null"),
        ("GIT_CONFIG_NOSYSTEM", "1"),
        ("HOME", home.as_str()),
        ("XDG_CONFIG_HOME", home.as_str()),
        ("GIT_AUTHOR_NAME", "Test Author"),
        ("GIT_AUTHOR_EMAIL", "author@example.invalid"),
        ("GIT_COMMITTER_NAME", "Test Committer"),
        ("GIT_COMMITTER_EMAIL", "committer@example.invalid"),
        ("GIT_AUTHOR_DATE", "2026-01-01T00:00:00Z"),
        ("GIT_COMMITTER_DATE", "2026-01-01T00:00:00Z"),
    ]
    .into_iter()
    .map(|(name, value)| (name.to_owned(), value.to_owned()))
    .collect()
}

fn violations(outcome: Result<Outcome, String>) -> Vec<String> {
    match outcome {
        Ok(Outcome::Checked(violations)) => violations,
        Ok(Outcome::Skipped(reason)) => panic!("skipped: {reason}"),
        Err(error) => panic!("failed: {error}"),
    }
}

/// A pull request run in CI against `base`, titled `feat: the change`.
fn pull_request(base: &str, description: &str) -> Environment {
    Environment {
        in_ci: true,
        base_branch: Some(base.to_owned()),
        github_ref: Some("refs/pull/1/merge".to_owned()),
        pull_request_title: Some("feat: the change".to_owned()),
        pull_request_body: Some(description.to_owned()),
    }
}

#[test]
fn local_run_checks_the_commits_since_main() {
    let scratch = Scratch::new("local-run", "main");
    scratch.commit_linked_test_and_spec();
    scratch.commit_test_change("chore: on main without trailer");
    scratch.git(&["switch", "--quiet", "--create", "feat/x"]);
    scratch.commit_test_change("refactor: on the branch without trailer");
    scratch.commit_test_change("refactor: explained\n\nSpec-Unchanged: comment only");

    let found = violations(scratch.run(&Environment::default()));

    assert_eq!(found.len(), 1, "{found:?}");
    assert!(
        found[0].contains("refactor: on the branch without trailer"),
        "{found:?}"
    );
    assert!(
        found[0].contains(TEST) && found[0].contains("DEMO-001"),
        "{found:?}"
    );
}

#[test]
fn pull_request_run_checks_the_commits_since_the_remote_base() {
    let scratch = Scratch::new("pull-request", "main");
    scratch.commit_linked_test_and_spec();
    let base = scratch.commit_test_change("chore: on main without trailer");
    scratch.git(&["update-ref", "refs/remotes/origin/main", &base]);
    scratch.git(&["switch", "--quiet", "--create", "feat/x"]);
    scratch.git(&["branch", "--quiet", "--delete", "main"]);
    scratch.commit_test_change("refactor: on the branch without trailer");

    let found = violations(scratch.run(&pull_request("main", "Spec-Unchanged: comment only")));

    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].contains("on the branch"), "{found:?}");
}

#[test]
fn pull_request_description_without_the_trailer_fails_the_squash_commit() {
    let scratch = Scratch::new("squash", "main");
    scratch.commit_linked_test_and_spec();
    let base = scratch.git(&["rev-parse", "HEAD"]);
    scratch.git(&["update-ref", "refs/remotes/origin/main", base.trim()]);
    scratch.git(&["switch", "--quiet", "--create", "feat/x"]);
    scratch.commit_test_change("refactor: explained\n\nSpec-Unchanged: comment only");
    scratch.write("README.md", "readme\n");
    scratch.commit("docs: add a readme");

    let found = violations(scratch.run(&pull_request("main", "Why the change.")));

    assert_eq!(found.len(), 1, "{found:?}");
    assert!(
        found[0].starts_with("pull request \"feat: the change\"")
            && found[0].contains("description")
            && found[0].contains("Spec-Unchanged"),
        "{found:?}"
    );
    let explained = pull_request("main", "Why the change.\n\nSpec-Unchanged: comment only");
    assert!(violations(scratch.run(&explained)).is_empty());
}

#[test]
fn local_run_judges_the_squash_commit_only_when_a_title_is_given() {
    let scratch = Scratch::new("squash-local", "main");
    scratch.commit_linked_test_and_spec();
    scratch.git(&["switch", "--quiet", "--create", "feat/x"]);
    scratch.commit_test_change("refactor: explained\n\nSpec-Unchanged: comment only");

    assert!(violations(scratch.run(&Environment::default())).is_empty());
    let reproduced = Environment {
        pull_request_title: Some("feat: the change".to_owned()),
        ..Environment::default()
    };
    assert_eq!(violations(scratch.run(&reproduced)).len(), 1);
}

#[test]
fn ci_pull_request_run_without_a_title_is_an_error() {
    let scratch = Scratch::new("no-title", "main");
    scratch.commit_linked_test_and_spec();
    let environment = Environment {
        pull_request_title: None,
        ..pull_request("main", "")
    };

    let error = scratch.run(&environment).err().expect("an error");

    assert!(error.contains("PULL_REQUEST_TITLE"), "{error}");
}

#[test]
fn run_on_main_checks_the_head_commit_alone() {
    let scratch = Scratch::new("head", "main");
    scratch.commit_linked_test_and_spec();
    scratch.commit_test_change("chore: older without trailer");
    scratch.commit_test_change("chore: newest without trailer");

    let found = violations(scratch.run(&Environment::default()));

    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].contains("newest"), "{found:?}");
}

#[test]
fn merge_commits_are_left_out() {
    let scratch = Scratch::new("merge", "main");
    scratch.commit_linked_test_and_spec();
    scratch.git(&["switch", "--quiet", "--create", "feat/x"]);
    let own = scratch.commit_test_change("refactor: x\n\nSpec-Unchanged: comment only");
    scratch.git(&["switch", "--quiet", "main"]);
    scratch.write("README.md", "readme\n");
    scratch.commit("docs: add a readme");
    scratch.git(&["switch", "--quiet", "feat/x"]);
    scratch.git(&[
        "merge",
        "--quiet",
        "--no-ff",
        "--message",
        "Merge main",
        "main",
    ]);

    assert_eq!(commits_since(&scratch.repository, "main"), Ok(vec![own]));
    assert!(violations(scratch.run(&Environment::default())).is_empty());
}

#[test]
fn local_run_without_main_is_skipped() {
    let scratch = Scratch::new("no-main", "feat/x");
    scratch.commit_linked_test_and_spec();

    let outcome = scratch.run(&Environment::default());

    assert!(
        matches!(outcome, Ok(Outcome::Skipped(ref reason)) if reason.contains("main")),
        "{:?}",
        outcome.map(|_| ())
    );
}

#[test]
fn ci_run_without_a_resolvable_base_is_an_error() {
    let scratch = Scratch::new("no-base", "feat/x");
    scratch.commit_linked_test_and_spec();

    let outcome = scratch.run(&pull_request("main", ""));

    let error = outcome.err().expect("an error");
    assert!(error.contains("origin/main does not exist"), "{error}");
}
