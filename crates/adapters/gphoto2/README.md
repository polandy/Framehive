<!--
SPDX-FileCopyrightText: 2026 Framehive contributors
SPDX-License-Identifier: GPL-3.0-or-later
-->

# adapter-gphoto2

Layer: `adapter`, port `files` (architecture §3)

## Purpose

Implements the `Cameras` port of `files` with libgphoto2 (ADR 0034): it lists the
cameras and phones connected over PTP or MTP and opens one as a read-only file system
whose root holds one folder per storage. It is the only crate with `unsafe` code for
libgphoto2. The DCIM scan of a camera is left to the import.

## Public API

| Item | Meaning |
|---|---|
| `Gphoto2Cameras` | `files::Cameras` backed by libgphoto2; `list` leaves out `disk:` ports, `open` returns a `files::ReadOnlyFileSystem` |

## Behaviour

- `open` selects the model that autodetection reports at the port, so it never opens
  another camera. A `disk:<folder>` port uses the `directory` driver ("Directory
  Browse"); any other port without a detected camera is `CameraError::Open`. Opening
  lists the storages once, so a camera that does not answer fails at once.
- A camera held by another program (libgphoto2's USB claim, lock and busy errors) is
  `CameraError::InUse` (ADR 0034 §6). Dropping the file system and every reader opened
  from it releases the camera.
- One mutex per camera serializes its calls; one process-wide lock guards loading and
  unloading drivers, because libgphoto2 and libltdl keep global state there.
- libgphoto2 keeps every listing it has made. When a folder or file is not found, the
  adapter forgets the listings and asks once more, so files added while the camera is
  open are found and a scan does not relist every folder.
- Readers fetch 1 MiB chunks from any offset with `gp_camera_file_read`, so reading the
  metadata of a RAW file does not transfer the whole file.

## Conformance

`tests/conformance.rs` runs `files::testing::check_cameras`, and with it the
`ReadOnlyFileSystem` suite, on a temporary folder served by the `directory` driver at
`disk:<folder>`. `tests/disk_port.rs` covers a missing folder and reads across chunks;
unit tests cover the `disk:` filter of `list` and the busy errors. The `directory` driver hides dotfiles and files with
unknown extensions, so hidden files on a camera (IMPORT-014.5) are tested with
`files::testing::MemoryCameras` only (ADR 0034 §3).

## Dependencies

- `domain`, `files`
- libgphoto2 2.5.34, a native library from the Nix flake, patched so that its
  `directory` driver reads part of a file
  (`nix/patches/libgphoto2-directory-read-file.patch`, ADR 0034 §3).
- `src/ffi.rs` declares the C functions by hand; `src/shim.c` reads the fields of
  libgphoto2's structs, so their layouts stay out of Rust. `build.rs` compiles the shim
  with `$CC` and `ar`.
