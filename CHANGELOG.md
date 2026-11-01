<!--
SPDX-FileCopyrightText: 2026 Framehive contributors
SPDX-License-Identifier: GPL-3.0-or-later
-->

# Changelog

All user-visible changes are listed here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow
[Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- Repository scaffold: Cargo workspace with the `domain` and `kernel` crates, pinned
  Rust toolchain, Nix development shell, and REUSE-compliant licensing
  (GPL-3.0-or-later).
- `cargo xtask ci` with the file, architecture, dependency and specification checks.
- Name templates (IMPORT-001): the import plan gives each imported file its folders
  and name from a name template, keeps the files of one asset together, and marks
  files that cannot be named or are dated by their modification time.
- Import run: the import scans a source folder with all its subfolders and lists what
  it leaves out and why (IMPORT-005), dates each file by its capture time or else its
  modification time, resolves name conflicts and skips files already imported
  (IMPORT-003), and copies each file with a verified copy through a `.part` file
  (IMPORT-004). A file that fails is reported and the import goes on.
- `framehive import <source> <destination>` (CLI-001): imports a folder or a mounted
  memory card into a library folder from the command line, with `--template`,
  `--on-conflict add_suffix|skip|fail` and `--dry-run`. It prints one line per file
  as soon as the file is done and a summary, creates a missing destination, rejects
  a destination inside the source, and exits with 0 (all done), 1 (a file failed or
  a folder could not be read) or 2 (the import did not start).
- Job codes (IMPORT-006): the `{job}` token of a name template inserts a job code,
  and `framehive import --job-code <text>` supplies it. A job code is trimmed and
  must have 1–64 characters, without `/` or control characters. A template with
  `{job}` and no job code, or an invalid job code, stops the command with exit code
  2; a job code the template does not use is accepted with a warning.
- Nix flake package: `nix run . -- import <source> <destination>` runs the
  `framehive` command and `nix profile install .` installs it, built with the pinned
  toolchain.
