// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Damaged configuration files never crash the reader, and every error points inside
//! the text (CORE-006.22, ADR 0037).
//!
//! A deterministic stand-in for fuzzing: the texts of CORE-006 are cut off, have a
//! character replaced, or have a YAML character inserted, by a small PRNG with a fixed
//! seed. A failing case prints its number and text, so it can be replayed.

#![allow(clippy::unwrap_used, reason = "tests may panic on bad fixtures")]

use std::panic::{AssertUnwindSafe, catch_unwind};

use adapter_yaml::SaphyrParser;
use config::{FileType, Key, Position, read};

const SEED: u64 = 0x5EED_CAFE_F00D_0006;
const CASES: usize = 5_000;

const BACKUP: &[Key] = &[Key::list_of_text("destinations", "Backup destinations").required()];
const KEYS: &[Key] = &[
    Key::text("destination", "Folder to import into"),
    Key::list_of_text("job_codes", "Job codes"),
    Key::whole_number("minimum_free_space_gib", "Free space to keep"),
    Key::boolean("verify", "Verify each copy"),
    Key::choice(
        "on_conflict",
        "Conflict policy",
        &["add_suffix", "skip", "fail"],
    ),
    Key::mapping("backup", "Backup copies", BACKUP),
];
const FILE_TYPE: FileType = FileType::new("test", 1, KEYS);

/// Valid and refused texts of CORE-006, so the mutants reach every path of the reader.
const TEXTS: [&str; 14] = [
    "schema_version: 1\n",
    "schema_version: 1\ndestination: /a\njob_codes: [Smith, \"2026\"]\n",
    "schema_version: 1\nbackup:\n  destinations:\n    - /b\n    - /c\n",
    "schema_version: 1\nminimum_free_space_gib: 20\nverify: true\non_conflict: skip\n",
    "schema_version: 1\ndestinaton: /a\n",
    "schema_version: 1\ndestination: /a\ndestination: /b\n",
    "schema_version: 1\ndestination: &target /a\njob_codes: [*target]\n",
    "schema_version: 1\nbackup:\n  <<: {destinations: [/b]}\n",
    "schema_version: 1\ndestination: !!str /a\n",
    "schema_version: 1\n---\nschema_version: 1\n",
    "schema_version: 1\ndestination: \"/a\n",
    "schema_version: 1\njob_codes:\n  - Smith\n  - ~\n",
    "# only a comment\n- a\n",
    "schema_version: 1\ndestination: |\n  /a\n  /b\njob_codes: {a: b}\n",
];
const YAML_CHARACTERS: &[char] = &[
    ':', '-', '[', ']', '{', '}', '&', '*', '!', '#', '|', '>', '\'', '"', '\n', '\r', ' ', '\0',
];

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

fn mutate(random: &mut Random, text: &str) -> String {
    let mut characters: Vec<char> = text.chars().collect();
    let at = random.below(characters.len());
    match random.below(3) {
        0 => characters.truncate(at),
        1 => characters[at] = YAML_CHARACTERS[random.below(YAML_CHARACTERS.len())],
        _ => characters.insert(at, YAML_CHARACTERS[random.below(YAML_CHARACTERS.len())]),
    }
    characters.into_iter().collect()
}

/// True when `position` is a character of `text` or just after its last one; YAML
/// also breaks lines at `\r`.
fn is_inside(position: Position, text: &str) -> bool {
    let lines = text.replace("\r\n", "\n").replace('\r', "\n");
    let Some(line) = lines.split('\n').nth(position.line.wrapping_sub(1)) else {
        return false;
    };
    (1..=line.chars().count() + 1).contains(&position.column)
}

/// spec: CORE-006.22
#[test]
fn damaged_files_never_crash_and_errors_point_inside_the_text() {
    let mut random = Random(SEED);
    for case in 0..CASES {
        let original = TEXTS[random.below(TEXTS.len())];
        let mut text = mutate(&mut random, original);
        if random.below(4) == 0 && !text.is_empty() {
            text = mutate(&mut random, &text);
        }
        let outcome = catch_unwind(AssertUnwindSafe(|| {
            read(&SaphyrParser::new(), &FILE_TYPE, "mutant.yaml", &text)
        }));
        let Ok(result) = outcome else {
            panic!("case {case} panicked on {text:?}");
        };
        if let Err(error) = result {
            assert!(
                is_inside(error.position, &text),
                "case {case}: {error} is outside {text:?}"
            );
        }
    }
}
