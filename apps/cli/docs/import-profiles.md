<!--
SPDX-FileCopyrightText: 2026 Framehive contributors
SPDX-License-Identifier: GPL-3.0-or-later
-->

# framehive-cli: import profiles

`framehive import --profile <name>` (CLI-010, `src/profiles.rs`) checks the name
(ADR 0032), finds the configuration folder (from the platform,
`XDG_CONFIG_HOME` and `HOME`) and reads
`profiles/<name>.yaml` through the `FileSystem` port with `import::ImportProfile::read`
and `adapter-yaml`, with `HOME` for `~`, before anything is scanned. Each setting comes
from the command line, else the profile, else the default of CLI-001; `--backup`
and `--video-backup` replace the profile's lists. Only `--job-code` gets the unused job code warning. Every
failure is `error: …` with exit code 2. `framehive config init` (`src/config_init.rs`)
writes `profiles/example.yaml` as `<file>.part` renamed without replacing, so an
existing file is kept (exit code 1).
