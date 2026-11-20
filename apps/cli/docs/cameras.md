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
`OpenedCamera::scan` gives. The camera is scanned and planned through
`ImportRun::with_source` on the command's thread, then imported by an `import::ImportJob`
in a job queue of its own (IMPORT-015) with `ImportJob::with_source`, named
`camera:<model>` in the import history, and no destination is checked against it. The
opened camera is a `cameras::SharedCamera`, a `ReadOnlyFileSystem` behind a lock, so the
job and the dialog of the guided import can both use it. After the format readiness a camera run prints
`import::CAMERA_FORMAT_READINESS_NOTE`. The opened camera is dropped, which releases it,
when the command ends. `framehive cameras` prints `<model>  <port>` per camera.

## Tests

Cameras are tested in the crate with `files::testing::MemoryCameras`, given to
`import_with` through `Surroundings::cameras` (`src/import_command/camera_tests.rs`,
`src/cameras.rs`). `tests/import_camera.rs` and, for a camera that cannot be used,
`tests/import_camera_failures.rs` run the binary against a folder that
libgphoto2's `directory` driver serves as a camera on a `disk:` port, and run
`camera:` alone and `framehive cameras`, with and without a camera, against libgphoto2's `vusb` virtual PTP camera, which serves a folder
(`VCAMERADIR`) as a Nikon D750 at `usb:001,001`. `IOLIBS` makes libgphoto2 load only
the port drivers in a folder below `FRAMEHIVE_TEST_IOLIBS`, which the Nix shell sets
(ADR 0034, amendment of 2026-11-19), so a camera connected to the machine is not seen.
A test that locks the folder with `flock` holds the virtual camera, which the patched
`vusb` driver then reports as claimed (amendment of 2026-11-20).
A test that sets `VCAMERA_UNPLUG_WHEN` to a path unplugs the virtual camera once that
file exists; it stays listed, as a camera that stops answering (CLI-014.10) does
(amendment of 2026-11-20). With `VCAMERA_CONNECTED_WHILE` as well, it is disconnected
instead and answers again once the test creates that file again (CLI-014.9).
Real PTP and MTP cameras need the manual hardware check (ADR 0034).
