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
