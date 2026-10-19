<!--
SPDX-FileCopyrightText: 2026 Framehive contributors
SPDX-License-Identifier: GPL-3.0-or-later
-->

# import

Layer: `module` (architecture §3, §4)

## Purpose

The import module brings files from a source (camera, card, phone, folder) into the
library. It decides where each file goes and what it is called, and describes that
decision as an import plan before anything is copied.

Its first feature is the name template (IMPORT-001): in the import plan, a name
template gives each asset file its folders and name, relative to the import
destination. All files of one asset get the same values, sidecars follow their file,
and compatibility mode keeps the names valid on exFAT, SMB and macOS.

Boundaries:

- **Pure logic.** Making an import plan does no I/O. Capture times, file modification
  times and the time the import was queued reach the module as values, through ports
  (`Clock` in `domain`, files and devices in the services); copying runs as a job.
- **Grouping** of files into assets is defined by LIBRARY-001; the module uses it and
  does not redefine it.
- **Not covered yet:** sequence numbers (IMPORT-002), name conflicts (IMPORT-003),
  metadata and job-code tokens, the live preview and block editor (a UI spec), and
  where a profile stores its template (the profile spec).

## Public API

| Item | Meaning |
|---|---|
| `MODULE_ID`, `module_id` | Stable id of the module (architecture §4.2) |
| `ExampleViewModel` | Example ViewModel from the template; replace it |

## Events

Published: none yet. Subscribed: none yet. New events need an ADR (architecture §4.5).

## Commands

Handled: none yet. Sent: none yet. New commands need an ADR (architecture §4.5).

## Data

No tables yet. Tables use the prefix `import_` and describe their rebuild
(architecture §4.4).

## Extension points

None yet.

## Dependencies

- `kernel`
