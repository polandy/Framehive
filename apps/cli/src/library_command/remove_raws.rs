// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! `framehive library remove-raws` (CLI-018): the summary, the confirmation and the run
//! of Remove RAWs on a folder.

use std::io::{BufRead, Write};
use std::path::Path;

use files::Trash;
use library::{ChangeReason, LibraryAsset, RemoveRawsPlan, RemoveRawsReport, RemoveRawsTotals};
use metadata::xmp::SidecarFiles;

use super::{read_folder_message, read_library_folder, write_line};
use crate::guided::size_text;
use crate::{FILES_FAILED, NOT_STARTED, write_error, write_hint, write_warning};

/// The answers that confirm, in any case.
const YES: [&str; 2] = ["y", "yes"];

/// The streams of the command, so the tests can give and read them.
pub(crate) struct Terminal<'a> {
    /// Standard input, for the answer.
    pub(crate) input: &'a mut dyn BufRead,
    /// Standard output.
    pub(crate) output: &'a mut dyn Write,
    /// Standard error.
    pub(crate) errors: &'a mut dyn Write,
}

/// The answer to the question whether to move the files.
enum Answer {
    Yes,
    No,
    /// Standard input ended, or could not be read, before a line was answered.
    Missing,
}

/// Runs Remove RAWs on the assets of `folder`, which is absolute, and returns the exit
/// code of CLI-018; `yes` moves the files without asking.
pub(crate) fn remove_raws(
    sidecars: SidecarFiles<'_>,
    trash: &dyn Trash,
    folder: &Path,
    dry_run: bool,
    yes: bool,
    terminal: &mut Terminal<'_>,
) -> u8 {
    let library_folder = match read_library_folder(sidecars, folder, terminal.errors) {
        Ok(library_folder) => library_folder,
        Err(error) => {
            write_error(terminal.errors, &read_folder_message(&error));
            return NOT_STARTED;
        }
    };
    let plan = RemoveRawsPlan::new(&library_folder.assets);
    write_summary(terminal.output, &plan);
    let files = plan.totals().files;
    if dry_run {
        for file in plan.files() {
            write_line(terminal.output, &format!("    {}", file.path.display()));
        }
        return 0;
    }
    if files == 0 {
        write_line(terminal.output, "Nothing to remove.");
        return 0;
    }
    let answer = if yes {
        Answer::Yes
    } else {
        ask(terminal, files)
    };
    match answer {
        Answer::Yes => {}
        Answer::No => {
            write_line(terminal.output, "Nothing moved.");
            return 0;
        }
        Answer::Missing => {
            write_error(terminal.errors, "no answer to the question; nothing moved");
            write_hint(
                terminal.errors,
                &format!("framehive library remove-raws --yes {}", folder.display()),
            );
            return NOT_STARTED;
        }
    }
    let report = plan.run(sidecars, trash);
    write_report_problems(terminal.errors, &report);
    let moved = totals_text(&report.moved);
    write_line(terminal.output, &format!("Moved to the trash: {moved}"));
    if report.not_moved.is_empty() && report.changed.is_empty() {
        0
    } else {
        FILES_FAILED
    }
}

/// The assets that changed since the summary, then the files that did not move.
fn write_report_problems(errors: &mut dyn Write, report: &RemoveRawsReport) {
    for changed in &report.changed {
        let message = format!(
            "{} changed since the summary: {}; its RAW stays",
            changed.asset.display(),
            change_text(changed.reason)
        );
        write_warning(errors, &message);
    }
    for not_moved in &report.not_moved {
        let message = format!(
            "cannot move {} to the trash: {}",
            not_moved.path.display(),
            not_moved.reason
        );
        write_error(errors, &message);
    }
}

fn change_text(reason: ChangeReason) -> &'static str {
    match reason {
        ChangeReason::RawGone => "its RAW is gone",
        ChangeReason::RenderedGone => "its rendered file is gone",
        ChangeReason::MarkedKeepRaw => "it is marked Keep RAW",
        ChangeReason::Rejected => "it is rejected",
        ChangeReason::StateUnreadable => "its state cannot be read",
    }
}

fn write_summary(output: &mut dyn Write, plan: &RemoveRawsPlan) {
    let skipped = plan.skipped();
    let lines = [
        format!("Remove RAWs: {}", totals_text(&plan.totals())),
        group_line("RAW will be removed", plan.removed()),
        group_line("RAW stays (Keep RAW)", plan.kept()),
        format!(
            "  Skipped: {} rejected, {} JPEG only, {} RAW only",
            skipped.rejected, skipped.rendered_only, skipped.raw_only
        ),
    ];
    for line in &lines {
        write_line(output, line);
    }
}

/// `  <title> (<count>): <base>, …`, the base names in name order, or `-` for none.
fn group_line<'a>(title: &str, assets: impl Iterator<Item = &'a LibraryAsset>) -> String {
    let mut names: Vec<&str> = assets.map(|asset| asset.asset.base_name()).collect();
    names.sort_unstable();
    let list = if names.is_empty() {
        "-".to_owned()
    } else {
        names.join(", ")
    };
    format!("  {title} ({}): {list}", names.len())
}

/// `<assets> assets, <files> files, <size> freed`.
fn totals_text(totals: &RemoveRawsTotals) -> String {
    format!(
        "{}, {}, {} freed",
        counted(totals.assets, "asset"),
        counted(totals.files, "file"),
        size_text(totals.bytes)
    )
}

/// `1 <noun>`, or the count with the plural.
fn counted(count: usize, noun: &str) -> String {
    if count == 1 {
        format!("1 {noun}")
    } else {
        format!("{count} {noun}s")
    }
}

/// Asks whether to move `files` files and reads the answer.
fn ask(terminal: &mut Terminal<'_>, files: usize) -> Answer {
    let question = format!("Move {} to the trash? [y/N] ", counted(files, "file"));
    // A question that cannot be shown still gets its answer read.
    let _ = write!(terminal.output, "{question}").and_then(|()| terminal.output.flush());
    let mut answer = String::new();
    match terminal.input.read_line(&mut answer) {
        Ok(0) | Err(_) => Answer::Missing,
        Ok(_) if is_yes(&answer) => Answer::Yes,
        Ok(_) => Answer::No,
    }
}

fn is_yes(answer: &str) -> bool {
    YES.iter()
        .any(|yes| answer.trim().eq_ignore_ascii_case(yes))
}
