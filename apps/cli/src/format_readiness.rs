// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The format readiness at the end of an import report (CLI-007): whether the source
//! can be formatted, and which files and folders keep it.

use import::{FormatBlocker, FormatReadiness};

/// The lines of the format readiness (CLI-007), naming the source as `noun`:
/// `framehive import` says `source`, the guided import says `card`. A dry run has no
/// lines, because it verifies nothing.
pub(crate) fn format_readiness_lines(readiness: &FormatReadiness, noun: &str) -> Vec<String> {
    if readiness.dry_run {
        return Vec::new();
    }
    let required = readiness.required_locations;
    let mut lines = vec![format_verdict_line(readiness, noun)];
    lines.extend(
        readiness
            .blockers
            .iter()
            .map(|blocker| format_blocker_line(blocker, required)),
    );
    if readiness.left_out_not_saved > 0 {
        lines.push(format!(
            "Not saved: {} left out by the scan.",
            readiness.left_out_not_saved
        ));
    }
    lines
}

/// The first line of the format readiness: whether the source can be formatted.
fn format_verdict_line(readiness: &FormatReadiness, noun: &str) -> String {
    let locations = locations(readiness.required_locations);
    if readiness.stopped {
        format!("Do not format the {noun}: the import was stopped.")
    } else if readiness.can_be_formatted() {
        format!("The {noun} can be formatted: every file is verified in {locations}.")
    } else {
        format!("Do not format the {noun}: not every file is verified in {locations}.")
    }
}

/// The indented line of a file or folder that keeps the source from being formatted.
fn format_blocker_line(blocker: &FormatBlocker, required_locations: usize) -> String {
    match blocker {
        FormatBlocker::NotSaved { path, verified_at } => format!(
            "  {}: verified in {verified_at} of {}",
            path.display(),
            locations(required_locations)
        ),
        FormatBlocker::LeftOut { path, reason } => {
            format!("  {}: left out: {reason}", path.display())
        }
    }
}

/// `<n> locations`, singular when `n` is 1.
fn locations(count: usize) -> String {
    if count == 1 {
        "1 location".to_owned()
    } else {
        format!("{count} locations")
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use import::LeftOutReason;

    use super::*;

    fn readiness(stopped: bool, blockers: Vec<FormatBlocker>) -> FormatReadiness {
        FormatReadiness {
            required_locations: 2,
            left_out_not_saved: 1,
            stopped,
            dry_run: false,
            blockers,
        }
    }

    /// spec: CLI-007.3
    #[test]
    fn the_format_readiness_lists_each_blocker_and_counts_what_the_scan_left_out() {
        let blockers = vec![
            FormatBlocker::NotSaved {
                path: PathBuf::from("DCIM/100CANON/IMG_0002.CR2"),
                verified_at: 1,
            },
            FormatBlocker::LeftOut {
                path: PathBuf::from("DCIM/101CANON"),
                reason: LeftOutReason::SidecarNeedsGrouping,
            },
        ];

        assert_eq!(
            format_readiness_lines(&readiness(false, blockers), "card"),
            [
                "Do not format the card: not every file is verified in 2 locations.",
                "  DCIM/100CANON/IMG_0002.CR2: verified in 1 of 2 locations",
                "  DCIM/101CANON: left out: sidecar, needs grouping",
                "Not saved: 1 left out by the scan.",
            ]
        );
    }

    #[test]
    fn a_stopped_run_is_not_ready_to_be_formatted() {
        let blockers = vec![FormatBlocker::NotSaved {
            path: PathBuf::from("DSCF0001.RAF"),
            verified_at: 1,
        }];

        assert_eq!(
            format_readiness_lines(&readiness(true, blockers), "source"),
            [
                "Do not format the source: the import was stopped.",
                "  DSCF0001.RAF: verified in 1 of 2 locations",
                "Not saved: 1 left out by the scan.",
            ]
        );
    }

    /// spec: CLI-007.7
    #[test]
    fn a_dry_run_has_no_format_readiness() {
        let mut dry_run = readiness(false, Vec::new());
        dry_run.dry_run = true;

        assert!(format_readiness_lines(&dry_run, "source").is_empty());
    }
}
