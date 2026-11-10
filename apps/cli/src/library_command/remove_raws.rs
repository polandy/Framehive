// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! `framehive library remove-raws` (CLI-018): the summary, the confirmation and the run
//! of Remove RAWs on a folder.

use std::io::{BufRead, Write};
use std::path::Path;

use files::Trash;
use library::{LibraryAsset, RemoveRawsPlan, RemoveRawsTotals};
use metadata::xmp::SidecarFiles;

use super::{read_library_folder, write_line};
use crate::guided::size_text;
use crate::{FILES_FAILED, NOT_STARTED, write_error};

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

/// Runs Remove RAWs on the assets of `folder`, which is absolute, and returns the exit
/// code of CLI-018.
pub(crate) fn remove_raws(
    sidecars: SidecarFiles<'_>,
    trash: &dyn Trash,
    folder: &Path,
    dry_run: bool,
    terminal: &mut Terminal<'_>,
) -> u8 {
    let library_folder = match read_library_folder(sidecars, folder, terminal.errors) {
        Ok(library_folder) => library_folder,
        Err(error) => {
            write_error(terminal.errors, &error.to_string());
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
    if !confirmed(terminal, files) {
        write_line(terminal.output, "Nothing moved.");
        return 0;
    }
    let report = plan.run(trash);
    for not_moved in &report.not_moved {
        let message = format!(
            "cannot move {} to the trash: {}",
            not_moved.path.display(),
            not_moved.reason
        );
        write_error(terminal.errors, &message);
    }
    let moved = totals_text(&report.moved);
    write_line(terminal.output, &format!("Moved to the trash: {moved}"));
    if report.not_moved.is_empty() {
        0
    } else {
        FILES_FAILED
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

/// Asks whether to move `files` files and reads the answer; the end of input or a
/// failed read is no.
fn confirmed(terminal: &mut Terminal<'_>, files: usize) -> bool {
    let question = format!("Move {} to the trash? [y/N] ", counted(files, "file"));
    // A question that cannot be shown still gets its answer read.
    let _ = write!(terminal.output, "{question}").and_then(|()| terminal.output.flush());
    let mut answer = String::new();
    match terminal.input.read_line(&mut answer) {
        Ok(0) | Err(_) => false,
        Ok(_) => YES
            .iter()
            .any(|yes| answer.trim().eq_ignore_ascii_case(yes)),
    }
}
