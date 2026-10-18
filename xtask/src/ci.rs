// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! `cargo xtask ci`: runs every check in the same order as CI and stops at the first
//! failure.
//!
//! The run is quiet: one line per passing step, and the full output only for the step
//! that fails. A long log costs reading time, and tokens when an agent reads it.

use std::io::Write;
use std::path::Path;
use std::process::{Command, Output};

/// An external command run by `ci`.
struct Step {
    title: &'static str,
    program: &'static str,
    arguments: &'static [&'static str],
}

const CARGO: &str = "cargo";

const STEPS: [Step; 10] = [
    Step {
        title: "format",
        program: CARGO,
        arguments: &["fmt", "--all", "--check"],
    },
    Step {
        title: "lint",
        program: CARGO,
        arguments: &[
            "clippy",
            "--workspace",
            "--all-targets",
            "--locked",
            "--",
            "-D",
            "warnings",
        ],
    },
    Step {
        title: "files",
        program: CARGO,
        arguments: &["xtask", "check-files"],
    },
    Step {
        title: "architecture",
        program: CARGO,
        arguments: &["xtask", "check-architecture"],
    },
    Step {
        title: "dependencies",
        program: CARGO,
        arguments: &["xtask", "check-dependencies"],
    },
    Step {
        title: "specifications",
        program: CARGO,
        arguments: &["xtask", "check-specs"],
    },
    Step {
        title: "tests",
        program: CARGO,
        arguments: &[
            "nextest",
            "run",
            "--workspace",
            "--locked",
            "--profile",
            "ci",
        ],
    },
    Step {
        title: "doc tests",
        program: CARGO,
        arguments: &["test", "--workspace", "--doc", "--locked"],
    },
    Step {
        title: "licenses and advisories",
        program: CARGO,
        arguments: &["deny", "check"],
    },
    Step {
        title: "REUSE compliance",
        program: "reuse",
        arguments: &["lint"],
    },
];

/// Runs all steps in `root`.
pub(crate) fn run(root: &Path) -> Result<(), String> {
    for step in &STEPS {
        let output = Command::new(program(step))
            .args(step.arguments)
            .current_dir(root)
            .output()
            .map_err(|error| missing_tool(step, &error))?;
        if !output.status.success() {
            print_failure(step, &output);
            return Err(format!(
                "step \"{}\" failed ({})",
                step.title, output.status
            ));
        }
        report_passed(step, &output);
    }
    Ok(())
}

/// Uses the cargo that runs xtask, so the pinned toolchain is used throughout.
fn program(step: &Step) -> String {
    if step.program == CARGO {
        std::env::var("CARGO").unwrap_or_else(|_| CARGO.to_owned())
    } else {
        step.program.to_owned()
    }
}

#[allow(
    clippy::print_stderr,
    reason = "xtask reports progress on the terminal"
)]
fn report_passed(step: &Step, output: &Output) {
    eprintln!("ok   {}", step.title);
    // Our own checks report warnings that do not fail the run; they must stay visible.
    if step.arguments.first() == Some(&"xtask") {
        for warning in String::from_utf8_lossy(&output.stderr)
            .lines()
            .filter(|line| line.starts_with("warning: "))
        {
            eprintln!("     {warning}");
        }
    }
}

/// Prints the failing command and everything it wrote, stdout first.
#[allow(
    clippy::print_stderr,
    reason = "xtask reports progress on the terminal"
)]
fn print_failure(step: &Step, output: &Output) {
    eprintln!(
        "FAIL {}: {} {}",
        step.title,
        step.program,
        step.arguments.join(" ")
    );
    let mut stderr = std::io::stderr().lock();
    // A broken terminal must not hide the real failure, so write errors are ignored.
    let _ = stderr.write_all(&output.stdout);
    let _ = stderr.write_all(&output.stderr);
}

fn missing_tool(step: &Step, error: &std::io::Error) -> String {
    format!(
        "cannot run {} for step \"{}\": {error}\nhint: run inside `nix develop`, which provides \
         every tool CI uses",
        step.program, step.title
    )
}
