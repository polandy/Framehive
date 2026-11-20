// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Test levels, read from where a marker sits, and the rule that every `CLI-*` criterion
//! has a test that runs the real binary (ADR 0010, amendment 2026-11-20).

use super::Report;
use super::document::Spec;
use super::markers::Marker;

/// Where the system tests of the command line live; they run the real binary.
const CLI_SYSTEM_TESTS: &str = "apps/cli/tests/";

/// How much of the program a test runs, from the outside in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Level {
    /// `apps/<app>/tests/`: the built program.
    System,
    /// `<crates|modules|xtask>/<crate>/tests/`: a crate through its public API.
    Integration,
    /// A `#[cfg(test)]` module inside a crate's sources.
    Unit,
}

impl Level {
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Integration => "integration",
            Self::Unit => "unit",
        }
    }
}

/// The level of a test at `location`, such as `apps/cli/tests/import.rs:12`.
pub(crate) fn level_of(location: &str) -> Level {
    let segments: Vec<&str> = location.split('/').collect();
    let crate_tests = matches!(
        segments.as_slice(),
        ["xtask", "tests", ..] | [_, _, "tests", ..]
    );
    match (crate_tests, segments.first()) {
        (true, Some(&"apps")) => Level::System,
        (true, _) => Level::Integration,
        (false, _) => Level::Unit,
    }
}

/// Every reachable `CLI-*` criterion has a test under `apps/cli/tests/`, unless the spec
/// lists it in `- E2e-owed:`; a listed criterion that has one makes the field stale.
pub(super) fn check_cli_system_tests(specs: &[Spec], markers: &[Marker], report: &mut Report) {
    let reachable = |spec: &&Spec| spec.status == "implemented" || spec.status == "partial";
    let cli = |spec: &&Spec| spec.id.starts_with("CLI-");
    for spec in specs.iter().filter(reachable).filter(cli) {
        let needs_test = |n: &&u32| !spec.removed.contains(n) && !spec.pending.contains(n);
        for number in spec.criteria.iter().filter(needs_test) {
            let has_system_test = markers.iter().any(|marker| {
                marker.spec_id == spec.id
                    && marker.number == *number
                    && marker.location.starts_with(CLI_SYSTEM_TESTS)
            });
            let owed = spec.e2e_owed.contains(number);
            let (file, id) = (&spec.file, &spec.id);
            if !has_system_test && !owed {
                report.errors.push(format!(
                    "{file}: criterion {id}.{number} has no test under {CLI_SYSTEM_TESTS}"
                ));
            } else if has_system_test && owed {
                report.errors.push(format!(
                    "{file}: criterion {id}.{number} has a test under {CLI_SYSTEM_TESTS}; \
                     drop it from `- E2e-owed:`"
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_level_from_the_path() {
        assert_eq!(level_of("apps/cli/tests/import.rs:3"), Level::System);
        assert_eq!(level_of("modules/import/tests/a.rs:3"), Level::Integration);
        assert_eq!(level_of("xtask/tests/a.rs:3"), Level::Integration);
        assert_eq!(
            level_of("apps/cli/src/guided/tests/start.rs:3"),
            Level::Unit
        );
        assert_eq!(level_of("crates/kernel/src/lib.rs:3"), Level::Unit);
    }

    fn cli_spec(owed: &[u32]) -> Spec {
        Spec {
            file: "specs/cli/CLI-009-x.md".to_owned(),
            id: "CLI-009".to_owned(),
            status: "implemented".to_owned(),
            criteria: vec![1, 2, 3],
            removed: vec![3],
            e2e_owed: owed.to_vec(),
            ..Spec::default()
        }
    }

    fn marker(number: u32, location: &str) -> Marker {
        Marker {
            location: location.to_owned(),
            spec_id: "CLI-009".to_owned(),
            number,
        }
    }

    fn errors(spec: Spec, markers: &[Marker]) -> Vec<String> {
        let mut report = Report::default();
        check_cli_system_tests(&[spec], markers, &mut report);
        report.errors
    }

    #[test]
    fn a_cli_criterion_with_only_a_unit_test_needs_a_system_test() {
        let markers = [
            marker(1, "apps/cli/tests/a.rs:1"),
            marker(2, "apps/cli/src/a.rs:1"),
        ];
        assert_eq!(
            errors(cli_spec(&[]), &markers),
            ["specs/cli/CLI-009-x.md: criterion CLI-009.2 has no test under apps/cli/tests/"]
        );
        assert!(errors(cli_spec(&[2]), &markers).is_empty());
    }

    #[test]
    fn an_e2e_owed_criterion_with_a_system_test_is_stale() {
        let markers = [
            marker(1, "apps/cli/tests/a.rs:1"),
            marker(2, "apps/cli/tests/b.rs:1"),
        ];
        assert_eq!(
            errors(cli_spec(&[2]), &markers),
            [
                "specs/cli/CLI-009-x.md: criterion CLI-009.2 has a test under apps/cli/tests/; \
                 drop it from `- E2e-owed:`"
            ]
        );
    }

    #[test]
    fn other_specs_need_no_system_test() {
        let mut spec = cli_spec(&[]);
        spec.id = "IMPORT-009".to_owned();
        assert!(errors(spec, &[]).is_empty());
    }
}
