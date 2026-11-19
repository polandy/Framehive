// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Fuzz target `configuration`: any text goes to `config::read` with the `saphyr`
//! adapter, for a file type with a key of every kind (CORE-006, ADR 0037).
//!
//! A panic, or an error at a position outside the text, is a finding (CORE-006.22).

#![no_main]

use adapter_yaml::SaphyrParser;
use config::{FileType, Key, read};
use libfuzzer_sys::fuzz_target;

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
const FILE_TYPE: FileType = FileType::new("fuzz", 1, KEYS);

fuzz_target!(|content: &[u8]| {
    let Ok(text) = std::str::from_utf8(content) else {
        return;
    };
    let Err(error) = read(&SaphyrParser::new(), &FILE_TYPE, "fuzz.yaml", text) else {
        return;
    };
    // YAML also breaks lines at `\r`.
    let lines = text.replace("\r\n", "\n").replace('\r', "\n");
    let line = lines
        .split('\n')
        .nth(error.position.line.wrapping_sub(1))
        .unwrap_or_else(|| panic!("{error}: line outside the text"));
    assert!(
        (1..=line.chars().count() + 1).contains(&error.position.column),
        "{error}: column outside the line"
    );
});
