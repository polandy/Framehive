<!--
SPDX-FileCopyrightText: 2026 Framehive contributors
SPDX-License-Identifier: GPL-3.0-or-later
-->

# framehive-cli: cameras

A source that starts with `camera:` names a camera (CLI-013, ADR 0034 §5): `camera:`
alone the only connected camera, text with a `:` a port, any other text a model,
ignoring ASCII case. `src/cameras.rs` finds and opens it through the `files::Cameras`
port, which `adapter-gphoto2` implements with libgphoto2; a camera that another program
holds stops the command with exit code 2 and a remedy, and nothing is ever stopped or
signalled. The camera's root `/` is the source: it is scanned with `import::scan_camera`
(only `DCIM` of each storage); `ScanError::NoStorage`, a camera without a storage
such as a locked phone, stops the command with exit code 2 and the remedy that
`OpenedCamera::scan` gives. The camera is read through `ImportRun::with_source`, named
`camera:<model>` in the import history with `ImportRun::with_source_name`, and no
destination is checked against it. After the format readiness a camera run prints
`import::CAMERA_FORMAT_READINESS_NOTE`. The opened camera is dropped, which releases it,
when the command ends. `framehive cameras` prints `<model>  <port>` per camera.

## Tests

Cameras are tested in the crate with `files::testing::MemoryCameras`, given to
`import_with` through `Surroundings::cameras` (`src/import_command/camera_tests.rs`,
`src/cameras.rs`). `tests/import_camera.rs` runs the binary against a folder that
libgphoto2's `directory` driver serves as a camera on a `disk:` port; PTP and MTP
cameras need the manual hardware check (ADR 0034).
