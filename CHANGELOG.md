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
  template step. Files imported before get no number (IMPORT-009).
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
- Import history in the import run (IMPORT-009): an import recognizes files it copied
  before by name, size, capture time and checksum, even after they were renamed or
  deleted in the library or the template changed. It does not copy them again, gives
  them no sequence number, reports them as `imported before`, and completes their
  missing backups. Each verified copy is recorded. `framehive import` and the guided
  import keep the history in `import-history.sqlite` in the state folder (CLI-008,
  ADR 0031), so a card imported again is recognized across runs, and an interrupted
  import is resumed by running it again. A history that cannot be used stops the
  import before any file is copied.
- `framehive import-log [<run>|last] [--format text|json|csv]` (CLI-009, IMPORT-010):
  lists the recorded import runs, oldest first, or prints which file of one run went
  where, with its checksum, as text, JSON or RFC 4180 CSV to redirect into a file. It
  reads the import history without creating or changing it.
- Import profiles (IMPORT-011, CLI-010, ADR 0032): `framehive import <source>
  --profile <name>` takes the destination, the backup destinations, the name template,
  the conflict policy and the job code from the YAML file `profiles/<name>.yaml` in the
  configuration folder (`$XDG_CONFIG_HOME/framehive` or `~/.config/framehive` on Linux,
  `~/Library/Application Support/framehive` on macOS). An option given on the command
  line replaces the profile's value, also for lists. A profile is read strictly: an
  unknown key, a wrong type or an invalid value stops the command with the file, line
  and column. `framehive config init` writes a commented example,
  `profiles/example.yaml`, and never replaces an existing one. The JSON Schema
  `schemas/import-profile-v1.json` gives completion in editors.
- Free-space check (IMPORT-012, CLI-011, ADR 0033): `framehive import` and the guided
  import check, before the first folder is created, that each file system can hold
  what the import writes to the destinations on it, counting only the files that are
  not there yet. A short file system is reported with what it needs and has free, and
  the import does not start (exit code 2; in the guided import, Enter does not start
  until `d` or `b` changes a folder). A dry run checks too. When the free space cannot
  be read, a warning says so and the import goes on.
- Video destinations (IMPORT-013, CLI-012): `framehive import --video-destination
  <folder>` and the profile key `video_destination` send videos to a folder of their own,
  and `--video-backup <folder>` (repeatable) and `backup.video_destinations` give them
  their own backup destinations; unset, videos go where photos go. Each video line of
  the report ends with `(video destination)`, the format readiness counts each file's
  own locations, and `framehive import-log` shows a run's video destination. The import
  history moves to layout 2 the first time it is written (ADR 0031).
- Cameras and phones over PTP and MTP (IMPORT-014, CLI-013, ADR 0034):
  `framehive import camera: <destination>` imports from the only connected camera, and
  `camera:<model>` or `camera:<port>` picks one of several; `framehive cameras` lists
  the connected cameras by model and port. Only the `DCIM` folder of each storage is
  scanned; the camera is read through libgphoto2, never written, and released when the
  command ends. A camera that another program holds (the file manager on Linux,
  `ptpcamerad` on macOS) stops the import with exit code 2 and says what to close. A
  camera that shows no storage, such as a locked phone or one set to charge only, also
  stops it with exit code 2 and says to unlock it and choose file transfer.
  After the format readiness, a camera run notes that only the files below `DCIM` were
  checked. The import log names the source `camera:<model>`. A folder whose name starts
  with `camera:` is given as `./camera:…`.
- Cameras in the guided import (CLI-014): `framehive` without a subcommand waits for a
  memory card or camera and offers a camera or phone connected over PTP or MTP like a
  card, by its model. A camera that another program holds or that shows no storage
  says what to do and asks `Try again?`. After the import the dialog asks
  `Release <model>?`. A camera disconnected during the import, or one that stops
  answering, stops the import like a pulled card; connected again, it is imported
  where it stopped without a question.
- Job queue (CORE-007, IMPORT-015, ADR 0035): imports now run as jobs in a job queue
  with one worker thread, first in, first out. `framehive import` and the guided import
  show the same output as before; a pulled card or camera still stops the import.
- Several sources in one command (CLI-015, IMPORT-015, ADR 0036):
  `framehive import <source>... --destination <folder>` imports each source as a job of its
  own. Jobs whose sources are on separate devices and whose destinations share no disk
  run at the same time; the others run one after another. Each job reserves its
  sequence numbers and target names when it is queued, so parallel imports never take
  the same ones. With several sources, each summary follows a line `<source>:`, and
  the exit code is the worst of the sources.

### Changed

- **Breaking:** `framehive import` names the destination with `--destination <folder>`
  (CLI-016, ADR 0036). Every folder on the command line is now a source, also with
  `--profile`; the destination is no longer the last folder. Without `--destination`
  the profile's destination is used; without either, the command stops with exit
  code 2, and after two or more folders a hint shows the command with
  `--destination`. A command without a source shows an example.

### Fixed

- Files already imported are recognized when the destination tells case or Unicode
  normalization forms apart and holds the same name in several spellings, such as
  `Café.JPG` precomposed and decomposed: every spelling is compared, so a photo
  identical to any of them is no longer copied again with a suffix (IMPORT-003).
- Files already imported are recognized when their target folder exists at the
  destination in another case or Unicode normalization form, such as `CAFÉ/` for
  `Café/`, at a destination that tells such names apart (IMPORT-003).
