// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! `cargo xtask ci-failures [<pr>]`: shows only what failed in the CI run of a pull
//! request.
//!
//! A full job log holds the Nix setup, the cache and every passing step. The part worth
//! reading is the output of `cargo xtask ci` from its `FAIL` line on, so this command
//! prints that part of each failed job and nothing else. It talks to GitHub through the
//! `gh` command line, which the Nix shell provides.

use std::path::Path;
use std::process::Command;

/// The line `cargo xtask ci` prints before the output of the failing step.
const FAILURE_MARKER: &str = "FAIL ";

/// Lines shown from the end of a failed job that has no `FAIL` line, such as a job
/// that broke while setting up Nix.
const TAIL_LINES: usize = 40;

/// Prints the failure of every failed check of the pull request, or of the pull request
/// of the current branch when `pull_request` is `None`.
pub(crate) fn run(root: &Path, pull_request: Option<&str>) -> Result<(), String> {
    let head = head_commit(root, pull_request)?;
    let failed = failed_jobs(root, &head)?;
    if failed.is_empty() {
        println!("no failed checks on {head}");
        return Ok(());
    }
    for job in &failed {
        let log = gh(root, &["run", "view", "--job", &job.id, "--log-failed"])?;
        println!("== {}", job.name);
        for line in failure_excerpt(&log) {
            println!("{line}");
        }
    }
    Err(format!("{} failed check(s) on {head}", failed.len()))
}

/// A failed check run; its ID is also the ID of the Actions job.
#[derive(Debug, PartialEq, Eq)]
struct FailedJob {
    id: String,
    name: String,
}

fn head_commit(root: &Path, pull_request: Option<&str>) -> Result<String, String> {
    let mut arguments = vec!["pr", "view"];
    arguments.extend(pull_request);
    arguments.extend(["--json", "headRefOid", "--jq", ".headRefOid"]);
    Ok(gh(root, &arguments)?.trim().to_owned())
}

fn failed_jobs(root: &Path, head: &str) -> Result<Vec<FailedJob>, String> {
    let endpoint = format!("repos/{{owner}}/{{repo}}/commits/{head}/check-runs");
    let listing = gh(
        root,
        &[
            "api",
            &endpoint,
            "--jq",
            r#".check_runs[] | select(.conclusion == "failure") | "\(.id)\t\(.name)""#,
        ],
    )?;
    Ok(parse_failed_jobs(&listing))
}

/// Reads the `<id>\t<name>` lines that `failed_jobs` asks `gh` for.
fn parse_failed_jobs(listing: &str) -> Vec<FailedJob> {
    listing
        .lines()
        .filter_map(|line| line.split_once('\t'))
        .map(|(id, name)| FailedJob {
            id: id.to_owned(),
            name: name.to_owned(),
        })
        .collect()
}

/// The part of a failed job's log worth reading: everything from the last `FAIL` line
/// of `cargo xtask ci` on, or else the last lines of the log.
fn failure_excerpt(log: &str) -> Vec<&str> {
    let lines: Vec<&str> = log.lines().map(message).collect();
    let start = lines
        .iter()
        .rposition(|line| line.starts_with(FAILURE_MARKER))
        .unwrap_or(lines.len().saturating_sub(TAIL_LINES));
    lines[start..].to_vec()
}

/// Strips the job, step and timestamp columns that `gh run view --log-failed` puts
/// before each line: `<job>\t<step>\t<timestamp> <message>`.
fn message(line: &str) -> &str {
    let after_columns = line.splitn(3, '\t').nth(2).unwrap_or(line);
    match after_columns.split_once(' ') {
        Some((timestamp, rest)) if is_timestamp(timestamp) => rest,
        _ => after_columns,
    }
}

/// GitHub writes RFC 3339 timestamps in UTC, such as `2026-10-18T19:29:53.8445367Z`.
fn is_timestamp(text: &str) -> bool {
    text.len() >= 20
        && text.as_bytes()[..4].iter().all(u8::is_ascii_digit)
        && text.as_bytes()[10] == b'T'
        && text.ends_with('Z')
}

fn gh(root: &Path, arguments: &[&str]) -> Result<String, String> {
    let output = Command::new("gh")
        .args(arguments)
        .current_dir(root)
        .output()
        .map_err(|error| {
            format!("cannot run gh: {error}\nhint: run inside `nix develop`, which provides gh")
        })?;
    if !output.status.success() {
        return Err(format!(
            "gh {} failed ({}):\n{}",
            arguments.join(" "),
            output.status,
            String::from_utf8_lossy(&output.stderr).trim_end()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn log_line(message: &str) -> String {
        format!("xtask ci (ubuntu-latest)\tcargo xtask ci\t2026-10-18T19:29:53.8445367Z {message}")
    }

    #[test]
    fn message_strips_job_step_and_timestamp_columns() {
        assert_eq!(message(&log_line("ok   format")), "ok   format");
    }

    #[test]
    fn message_keeps_a_line_without_columns() {
        assert_eq!(message("plain text"), "plain text");
    }

    #[test]
    fn message_keeps_a_first_word_that_is_not_a_timestamp() {
        assert_eq!(message("job\tstep\terror: boom"), "error: boom");
    }

    #[test]
    fn excerpt_starts_at_the_fail_line_of_xtask_ci() {
        let log = [
            log_line("ok   format"),
            log_line("FAIL lint: cargo clippy"),
            log_line("error: unused variable"),
        ]
        .join("\n");
        assert_eq!(
            failure_excerpt(&log),
            ["FAIL lint: cargo clippy", "error: unused variable"]
        );
    }

    #[test]
    fn excerpt_without_a_fail_line_is_the_tail_of_the_log() {
        let lines: Vec<String> = (0..50).map(|n| log_line(&format!("line {n}"))).collect();
        let log = lines.join("\n");
        let excerpt = failure_excerpt(&log);
        assert_eq!(excerpt.len(), TAIL_LINES);
        assert_eq!(excerpt.first(), Some(&"line 10"));
        assert_eq!(excerpt.last(), Some(&"line 49"));
    }

    #[test]
    fn failed_jobs_are_read_from_id_and_name() {
        assert_eq!(
            parse_failed_jobs("117768034035\txtask ci (macos-latest)\n\n"),
            [FailedJob {
                id: "117768034035".to_owned(),
                name: "xtask ci (macos-latest)".to_owned(),
            }]
        );
    }
}
