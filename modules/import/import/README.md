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

Its first feature is the name template (IMPORT-001): `ImportPlan` renders a name
template from the naming service for each asset file, giving its folders and name
relative to the import destination. All files of one asset get the same values: the
capture time chosen by `naming::asset_date`, and the base name of the asset's first
file that is not a sidecar, so sidecars follow their file. A file that cannot be named
is marked with the reason, and a file whose asset has no capture time is marked as
dated by its modification time. An asset without files adds nothing to the plan; an
asset of only sidecars takes its base name from its first file.

Boundaries:

- **Pure logic.** Making an import plan does no I/O. Capture times, file modification
  times and the time the import was queued reach the module as values, through ports
  (`Clock` in `domain`, files and devices in the services); copying runs as a job.
- **Grouping** of files into assets is defined by LIBRARY-001; the module uses it and
  does not redefine it. `ImportPlan::new` takes assets that are already grouped.
- **Not covered yet:** sequence numbers (IMPORT-002), name conflicts (IMPORT-003),
  metadata and job-code tokens, the live preview and block editor (a UI spec), and
  where a profile stores its template (the profile spec).

## Public API

| Item | Meaning |
|---|---|
| `MODULE_ID`, `module_id` | Stable id of the module (architecture §4.2) |
| `ImportPlan` | Pure description of which file goes where, with what name (IMPORT-001) |
| `AssetFile` | One file of an asset as the import finds it: name, role, capture and modification time |
| `PlannedFile` | The target path of one file, or why it cannot be named, and its date source |
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

- `domain`
- `kernel`
- `naming`
