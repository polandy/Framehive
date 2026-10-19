<!--
SPDX-FileCopyrightText: 2026 Framehive contributors
SPDX-License-Identifier: GPL-3.0-or-later
-->

# naming

Layer: `service` (architecture §2, §3)

## Purpose

The naming service holds the template language for file and folder names
(IMPORT-001). It validates a name template when it is saved, with the column of the
first error, and renders it for one asset file into a path relative to the import
destination.

Boundaries:

- **Pure.** Rendering is a function of the values it is given: capture time, import
  time, base name and file name. The service reads no files and no clock; the caller
  passes times as `LocalDateTime` values (architecture §3).
- **Asset values.** All files of one asset are rendered from one `AssetValues`.
  `asset_date` chooses its capture time: the RAW file first, then the other files in
  order, falling back to the modification time.
- **Sidecars.** A file's extension is everything after the asset's base name and a
  dot, so `DSC_0042.RAF.xmp` keeps `RAF.xmp` and follows its RAF file.
- **Not covered yet:** sequence tokens are validated, but rendering them returns
  `RenderError::SequenceUnavailable` until IMPORT-002. The import plan, which marks
  files that cannot be named or are dated by their modification time, belongs to the
  import module.

## Public API

| Item | Meaning |
|---|---|
| `NameTemplate` | Validated name template; `parse`, `as_str`, `render` |
| `TemplateError`, `TemplateErrorKind` | Why and at which column a template was rejected |
| `AssetValues` | The values all files of one asset are rendered from |
| `CompatibilityMode` | Keep names valid on exFAT, SMB and macOS (`On` by default) or not |
| `RenderError` | Why one file cannot be named |
| `LocalDateTime`, `DateTimeError` | Date and wall-clock time without a time zone |
| `FileTimes`, `AssetDate`, `DateSource`, `asset_date` | Choosing the capture time of an asset from its files |

## Dependencies

- `thiserror` (ADR 0016)
