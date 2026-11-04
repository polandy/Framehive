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
  time, base name, job code, sequence number and file name. The service reads no files and no clock; the caller
  passes times as `LocalDateTime` and `CaptureTime` values of `domain` (architecture §3,
  ADR 0021).
- **Asset values.** All files of one asset are rendered from one `AssetValues`.
  `asset_date` chooses its capture time from each file's `AssetFileRole`: the RAW file first, then the other files in
  order, falling back to the modification time.
- **Sidecars.** A file's extension is everything after the asset's base name and a
  dot, so `DSC_0042.RAF.xmp` keeps `RAF.xmp` and follows its RAF file.
- **Job code.** `{job}` inserts `AssetValues::job_code` as given; validating it is the
  import module's job (IMPORT-006). Without a job code, a file of a template with
  `{job}` cannot be named (`RenderError::NoJobCode`); `uses_job_code` lets the caller
  refuse such an import before it starts.
- **Sequence numbers.** A sequence token renders `AssetValues::sequence_number`
  with a fixed number of digits or as letters `a`…`zz` (IMPORT-002). The caller
  reserves the numbers; `check_sequence_range` refuses a range whose last number does
  not fit, before the import starts, and suggests a template that fits.
- **Not here:** the import plan, which marks files that cannot be named or are dated
  by their modification time, belongs to the import module.

## Public API

| Item | Meaning |
|---|---|
| `NameTemplate` | Validated name template; `parse`, `as_str`, `uses_job_code`, `sequence_kind`, `check_sequence_range`, `render` |
| `SequenceKind` | Which counter a sequence token numbers from, and whether it is persisted |
| `SequenceOverflow` | A range that does not fit the sequence token, with a suggested template |
| `TemplateError`, `TemplateErrorKind` | Why and at which column a template was rejected |
| `AssetValues` | The values all files of one asset are rendered from |
| `CompatibilityMode` | Keep names valid on exFAT, SMB and macOS (`On` by default) or not |
| `RenderError` | Why one file cannot be named |
| `FileTimes`, `AssetDate`, `DateSource`, `asset_date` | Choosing the capture time of an asset from its files |

## Dependencies

- `domain`
- `thiserror` (ADR 0016)
