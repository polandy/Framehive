// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Damaged name templates never crash validation or rendering, and an accepted one
//! never renders a path that leaves the destination (IMPORT-001.21, ADR 0037).
//!
//! A deterministic stand-in for fuzzing: the templates of IMPORT-001 are cut off, have
//! a character replaced, or have a template character inserted, by a small PRNG with a
//! fixed seed. A failing case prints its number and template, so it can be replayed.

#![allow(clippy::unwrap_used, reason = "tests may panic on bad fixtures")]

use std::panic::{AssertUnwindSafe, catch_unwind};

use domain::LocalDateTime;
use naming::{AssetValues, CompatibilityMode, NameTemplate};

const SEED: u64 = 0x5EED_CAFE_F00D_0001;
const CASES: usize = 5_000;

/// The templates of IMPORT-001, so the mutants reach every token and option.
const TEMPLATES: [&str; 11] = [
    "{date:%Y}/{date:%Y-%m-%d}/{date:%Y%m%d}-{name}.{ext}",
    "{{draft}}_{name}.{ext}",
    "{name:lower}.{ext:upper}",
    "{number}.{ext}",
    "{number:2}-{number:6}.{ext}",
    "{imported:%Y-%m-%d}/{date:%H:%M:%S}/{name}.{ext}",
    "{seq_today:3}-{name}.{ext}",
    "{date:%H:%M} {name}. /{name}.{ext}",
    "{date:%Y}/{job}/{camera}/{name}.{ext}",
    "{seq_session:4}_{name:upper}.{ext:lower}",
    "{seq_letters}-{name}.{ext}",
];
const TEMPLATE_CHARACTERS: &[char] = &['{', '}', ':', '%', '/', '.', ' ', '_'];

/// Xorshift64: small, fast and the same on every platform.
struct Random(u64);

impl Random {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    /// A number below `bound`, which must not be zero.
    fn below(&mut self, bound: usize) -> usize {
        usize::try_from(self.next() % u64::try_from(bound).unwrap()).unwrap()
    }
}

fn mutate(random: &mut Random, template: &str) -> String {
    let mut characters: Vec<char> = template.chars().collect();
    let at = random.below(characters.len());
    let character = TEMPLATE_CHARACTERS[random.below(TEMPLATE_CHARACTERS.len())];
    match random.below(3) {
        0 => characters.truncate(at),
        1 => characters[at] = character,
        _ => characters.insert(at, character),
    }
    characters.into_iter().collect()
}

fn values() -> AssetValues {
    AssetValues {
        base_name: "DSC_0042".to_owned(),
        capture_time: LocalDateTime::new(2026, 10, 17, 14, 5, 9).unwrap(),
        import_time: LocalDateTime::new(2026, 10, 19, 9, 0, 0).unwrap(),
        job_code: Some("Smith: wedding".to_owned()),
        sequence_number: Some(7),
        camera_model: Some("X-T5".to_owned()),
    }
}

fn stays_inside_the_destination(path: &str) -> bool {
    path.split('/')
        .all(|segment| !matches!(segment, "" | "." | ".."))
}

/// Validation, then rendering in both compatibility modes.
fn check(case: usize, template: &str) {
    let parsed = match NameTemplate::parse(template) {
        Ok(parsed) => parsed,
        Err(error) => {
            let columns = 1..=template.chars().count() + 1;
            assert!(
                columns.contains(&error.column),
                "case {case}: column {} is outside {template:?}",
                error.column
            );
            return;
        }
    };
    for mode in [CompatibilityMode::On, CompatibilityMode::Off] {
        if let Ok(path) = parsed.render(&values(), "DSC_0042.RAF", mode) {
            assert!(
                stays_inside_the_destination(&path),
                "case {case}: {template:?} gives {path:?}"
            );
        }
    }
}

/// spec: IMPORT-001.21
#[test]
fn damaged_templates_never_crash_and_never_leave_the_destination() {
    for template in TEMPLATES {
        assert!(
            NameTemplate::parse(template).is_ok(),
            "{template:?} is a valid seed"
        );
    }
    let mut random = Random(SEED);
    for case in 0..CASES {
        let original = TEMPLATES[random.below(TEMPLATES.len())];
        let mut template = mutate(&mut random, original);
        if random.below(4) == 0 && !template.is_empty() {
            template = mutate(&mut random, &template);
        }
        let outcome = catch_unwind(AssertUnwindSafe(|| check(case, &template)));
        assert!(outcome.is_ok(), "case {case} panicked on {template:?}");
    }
}
