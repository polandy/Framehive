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
  (IMPORT-003), treating names that differ only in case or Unicode normalization form
  as the same name (macOS and SMB shares may store names decomposed), and copies each
  file with a verified copy through a `.part` file (IMPORT-004). A file that fails is
  reported and the import goes on.
- `framehive import <source> <destination>` (CLI-001): imports a folder or a mounted
  memory card into a library folder from the command line, with `--template`,
  `--on-conflict add_suffix|skip|fail` and `--dry-run`. It prints one line per file
  as soon as the file is done and a summary, creates a missing destination, rejects
  a destination inside the source, and exits with 0 (all done), 1 (a file failed or
  a folder could not be read) or 2 (the import did not start).
- Job codes (IMPORT-006): the `{job}` token of a name template inserts a job code,
  and `framehive import --job-code <text>` supplies it. A job code is trimmed and
  must have 1–64 characters, without `/` or control characters, and not dots only. A template with
  `{job}` and no job code, or an invalid job code, stops the command with exit code
  2; a job code the template does not use is accepted with a warning.
- Nix flake package: `nix run . -- import <source> <destination>` runs the
  `framehive` command and `nix profile install .` installs it, built with the pinned
  toolchain.
- Guided import (CLI-002, CLI-003): `framehive` without a subcommand starts a line-based
  dialog in the terminal. It asks once for the library folder, waits for a memory
  card, offers three template presets with a preview of the card's first files, asks
  for a job code only when the template has `{job}`, and shows a plan summary before
  it imports with a progress line. It reports failed and skipped files, offers to
  eject the card, and stops between files when the card is pulled, going on when it
  is inserted again. The library folder, the template and the recent job codes are
  remembered in `$XDG_STATE_HOME/framehive` (IMPORT-006).
- Sequence numbers (IMPORT-002, CLI-004): name templates number assets in shooting
  order with `{seq_session:N}`, `{seq_letters}`, `{seq_today:N}` and
  `{seq_stored:N}`. The imported-today and stored counters are kept in
  `$XDG_STATE_HOME/framehive/sequence-counters.txt` and reserved when an import
  starts, so two imports never share a number; a dry run and the plan summary show the
  numbers without reserving them. Numbers that do not fit the template stop the
  import with a suggested template, and in the guided import lead back to the
  template step. With a persisted counter, files already imported are not recognized,
  and both imports say so.
- Backup destinations (IMPORT-007, CLI-005, CLI-006): `framehive import --backup <folder>`,
  repeatable, writes each copied or already imported file to the same path below every
  backup destination as well. The card is read once, every copy is verified on its
  own, and a backup target is never overwritten. The report has one line per backup
  destination for each file and a summary line for each; a failed backup makes the
  exit code 1, and a backup destination that is, contains or lies inside the
  destination, lies inside the source or is given twice stops the command. The guided
  import shows the backup destinations in the plan summary, adds or removes them with
  `b`, remembers them in the state file (now format version 2; version 1 is still
  read), refuses to start with one on the card, and reports failed backups.
- Format readiness (IMPORT-008, CLI-007): after an import, `framehive import` and the
  guided import say whether the source or card can be formatted, that is whether every
  file is verified at the destination and at every backup destination. Otherwise they
  list each file that is not saved, with the number of locations it is verified at, and
  each unreadable or ungrouped entry the scan left out. The guided import shows it before
  the eject question; a dry run shows nothing, and exit codes are unchanged.

### Fixed

- Files already imported are recognized when the destination tells case or Unicode
  normalization forms apart and holds the same name in several spellings, such as
  `Café.JPG` precomposed and decomposed: every spelling is compared, so a photo
  identical to any of them is no longer copied again with a suffix (IMPORT-003).
- Files already imported are recognized when their target folder exists at the
  destination in another case or Unicode normalization form, such as `CAFÉ/` for
  `Café/`, at a destination that tells such names apart (IMPORT-003).
