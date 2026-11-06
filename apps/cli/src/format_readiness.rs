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
    let mut lines = vec![format_verdict_line(readiness, noun)];
    lines.extend(readiness.blockers.iter().map(format_blocker_line));
    if readiness.left_out_not_saved > 0 {
        lines.push(format!(
            "Not saved: {} left out by the scan.",
            readiness.left_out_not_saved
        ));
    }
    lines
}

/// The first line of the format readiness: whether the source can be formatted. When
/// the files require different numbers of locations, as photos and videos can
/// (IMPORT-013), no single number is true for every file, so the line names none.
fn format_verdict_line(readiness: &FormatReadiness, noun: &str) -> String {
    let locations = if readiness.required_locations_differ {
        "all its locations".to_owned()
    } else {
        locations(readiness.required_locations)
    };
    if readiness.stopped {
        format!("Do not format the {noun}: the import was stopped.")
    } else if readiness.can_be_formatted() {
        format!("The {noun} can be formatted: every file is verified in {locations}.")
    } else {
        format!("Do not format the {noun}: not every file is verified in {locations}.")
    }
}

/// The indented line of a file or folder that keeps the source from being formatted.
/// A file that is not saved names the locations it requires itself (IMPORT-013).
fn format_blocker_line(blocker: &FormatBlocker) -> String {
    match blocker {
        FormatBlocker::NotSaved {
            path,
            verified_at,
            required_at,
        } => format!(
            "  {}: verified in {verified_at} of {}",
            path.display(),
            locations(*required_at)
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
            required_locations_differ: false,
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
                required_at: 2,
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
            required_at: 2,
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

    /// spec: IMPORT-013.3
    #[test]
    fn each_file_names_its_own_locations_when_the_files_differ() {
        let blockers = vec![
            FormatBlocker::NotSaved {
                path: PathBuf::from("DSCF0001.RAF"),
                verified_at: 1,
                required_at: 2,
            },
            FormatBlocker::NotSaved {
                path: PathBuf::from("clip.MOV"),
                verified_at: 2,
                required_at: 3,
            },
        ];
        let mut differing = readiness(false, blockers);
        differing.required_locations = 3;
        differing.required_locations_differ = true;
        differing.left_out_not_saved = 0;

        assert_eq!(
            format_readiness_lines(&differing, "source"),
            [
                "Do not format the source: not every file is verified in all its locations.",
                "  DSCF0001.RAF: verified in 1 of 2 locations",
                "  clip.MOV: verified in 2 of 3 locations",
            ]
        );
    }
}
