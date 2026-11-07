<!--
SPDX-FileCopyrightText: 2026 Framehive contributors
SPDX-License-Identifier: GPL-3.0-or-later
-->

# framehive-cli: tests

`tests/` runs the real binary (`CARGO_BIN_EXE_framehive`) in a temporary folder with
`TZ=UTC`, with test files from `metadata::testing`. Unreadable files and folders use
mode 000; when the tests run as root, which reads them anyway, the expectation
follows what the file system allows. `tests/import_profiles.rs` reads import profiles (IMPORT-011) with the real YAML
parser, including every profile shown in `docs/` and `specs/` and the example of
`framehive config init`. `tests/import_profile_option.rs` and `tests/config_init.rs`
(CLI-010) set `HOME` to a temporary folder and unset `XDG_CONFIG_HOME`. `tests/memory_card.rs` checks the milestone 1
guarantees end to end on a folder laid out like a memory card: a dry run writes
nothing, the import fails only the unreadable file, a second run copies nothing, and
the source keeps its paths, bytes and modification times. `tests/import_log.rs` reads
the log after real imports, or of a history written with `adapter-sqlite`. `tests/import_sources.rs`
(CLI-015) and `tests/import_source_checks.rs` (CLI-016) run the binary in the temporary
folder, so the sources are named as given.
