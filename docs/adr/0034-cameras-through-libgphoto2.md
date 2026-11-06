# 0034 – Cameras and phones through libgphoto2

- Status: accepted
- Date: 2026-11-06

## Context

Cameras and phones that are connected by USB do not appear as a mounted card. They
speak the Picture Transfer Protocol (PTP) or its extension, the Media Transfer Protocol
(MTP). Framehive must import from them on Linux and macOS (Q2, an MVP requirement since
2026-10-17). ADR 0023 left them out and named gphoto2 as a further source that needs its
own ADR, because it is a native library.

The import reads its source through the port `files::FileSystem` (IMPORT-004, IMPORT-005):
it lists folders, reads sizes and modification times, and opens files for reading from
any offset, because the metadata readers seek to the byte ranges they need (ADR 0021).
The same port also creates, renames and removes files. A camera can do the first set of
operations and must never get the second.

libgphoto2 is the library that Linux desktops, darktable, digiKam and Rapid Photo
Downloader use for PTP and MTP. It drives about 2,500 cameras and phones, is LGPL-2.1
(compatible with ADR 0014), and nixpkgs builds it for Linux and macOS (2.5.34). Its
`directory` camera driver serves a folder through the `disk:` port as if it were a
camera, so the whole path from the library to the import can be tested without hardware.

On Linux, the file manager's gvfs may already hold a connected camera; on macOS,
`ptpcamerad` does. libgphoto2 then cannot claim the USB device.

## Decision

### 1. A read-only port for the source

The `files` service splits the reading operations of `FileSystem` into a port
`ReadOnlyFileSystem`: `entry`, `list_folder`, `file_size`, `modification_time` and
`open_read`, with the same contracts. `FileSystem` extends it with the writing
operations, so `adapter-fs` and `MemoryFileSystem` implement both without change in
behaviour. The import reads its source only through `ReadOnlyFileSystem`; that the
import never writes to the source is then a property of the types. The port gets its
own conformance suite, which the `FileSystem` suite runs as well.

### 2. A `Cameras` port in the `files` service

The `files` service gets a port `Cameras`, because it holds the ports through which
Framehive reaches storage devices, and an adapter may depend on one service only
(architecture §3):

- `list` returns the connected cameras, each with its model name and its port (for
  example `usb:001,005`), sorted by port in byte order.
- `open` takes a port and returns the camera as a `ReadOnlyFileSystem`, or an error
  that tells a camera that is in use by another program from other failures.

The camera's file system has the root `/` and one folder per storage, named as the
camera reports it (for example `store_00010001`); a camera with two card slots has two.
Dropping the opened camera releases it. `testing::MemoryCameras` serves
`MemoryFileSystem`s as cameras, with injectable failures, for the import and the CLI.

### 3. An FFI adapter `adapter-gphoto2` with own declarations

A new adapter crate `crates/adapters/gphoto2` (`ffi = true`) implements `Cameras` with
libgphoto2:

- The C functions it calls (about fifteen: context, camera list and autodetect,
  port info, camera init and exit, folder and file listing, file info, partial file
  read, error text) are declared by hand in `src/ffi.rs`, as `adapter-libraw` does
  (ADR 0028 §5). No bindgen, no pkg-config crate, no `gphoto2` crate. libgphoto2's
  structs are used only behind pointers, so their layout stays out of Rust.
- `open_read` reads with `gp_camera_file_read` from the offset the reader asks for, in
  chunks, so a seek costs no full transfer and a large video is never held in memory.
  A driver that cannot read part of a file fails that file with the driver's message.
- One libgphoto2 camera is not safe to use from two threads; the adapter serializes
  every call to one camera with a mutex.
- Every `unsafe` block has a `// SAFETY:` comment; the conformance suites run against
  the `directory` driver on a temporary folder, through the port `disk:<folder>`.

The flake adds nixpkgs `libgphoto2` to the shell's build inputs. No Rust crate is
added, so `dependencies.yaml` does not change.

**Amendment (2026-11-06): a patched `directory` driver.** In libgphoto2 2.5.34 only
the `ptp2` driver can read part of a file; the `directory` driver has no
`read_file_func`, so `gp_camera_file_read` fails there with `GP_ERROR_NOT_SUPPORTED`,
and the conformance suites could not test `open_read`. The flake therefore applies
`nix/patches/libgphoto2-directory-read-file.patch` to nixpkgs' libgphoto2. It adds a
`read_file_func` of about forty lines to the `directory` driver that reads from an offset
with `open`, `lseek` and `read`, written so that it can be offered upstream. The tests
then take the same chunked read path through `gp_camera_file_read` that a PTP camera
takes. The patch changes only cameras on `disk:` ports, which `Cameras::list` never
returns, so the cameras a person imports from behave as with an unpatched library.
Options not taken:

- **Fetch the whole file into a temporary file when a partial read is not supported:**
  works with an unpatched library, but the tests would then cover this fallback instead
  of the chunked path that real cameras take, and the fallback would need a temporary
  folder and space for the largest video.
- **Leave `open_read` out of the tests on the `directory` driver:** no patch, but the
  read path, the part of the adapter most likely to be wrong, would be tested only by
  the manual hardware check before a release.

The `directory` driver also hides files whose names start with a dot and files whose
extension it does not know, such as `.txt`, `.HEIC` and `.HIF`. The adapter's tests
therefore use only names it lists. That the scan of a camera leaves out hidden files and
files of an unsupported type with their reasons (IMPORT-014.5) is tested through
`testing::MemoryCameras` only.

### 4. Only `DCIM` is scanned on a camera

Unlike a card folder (IMPORT-005), a camera source is scanned only below the `DCIM`
folder of each storage. A phone keeps music, downloads and app data next to `DCIM`;
a camera keeps only `DCIM` and its own system folders. The paths of the found files
start with the storage folder, for example `store_00010001/DCIM/100_FUJI/DSCF0001.RAF`.
Configurable scanned folders (Q6) and deduplication across two slots (Q8) come later.

### 5. A camera is named on the command line by `camera:`

`framehive import camera:[<model>|<port>] <destination>` imports from a camera:
`camera:` alone takes the only connected camera, text with a `:` is a port and is opened
directly, any other text is a model name. `framehive cameras` lists the connected
cameras. A port is opened even if autodetection does not list it, which is how the CLI
tests reach the `directory` driver with `camera:disk:<folder>`. The guided import offers
cameras in a later change, which generalizes the identity of a `Device` beyond its mount
point (ADR 0023).

### 6. A busy camera is reported, never taken

When libgphoto2 cannot claim the camera, the command stops with the cause and a remedy
(close the program that holds it). Framehive never stops gvfs, `ptpcamerad` or any other
process.

## Alternatives

- **Run the `gphoto2` program as a child process:** no FFI, but its text output must be
  parsed, every file costs a process, and it cannot read part of a file, so every
  metadata read would transfer the whole RAW file.
- **Import from the gvfs mount (`/run/user/<uid>/gvfs/gphoto2:…`):** no own code, but
  Linux only, slow, and only when the desktop runs gvfs.
- **The `gphoto2` crate:** a safe wrapper, but maintained by one person and not an
  established product (ADR 0009); it also pulls in bindgen-generated bindings.
- **bindgen at build time:** needs libclang in the build and generates thousands of
  declarations for fifteen functions.
- **A read-only `FileSystem` whose writing operations fail:** no port split, but the
  promise not to write to a camera would rest on runtime errors.
- **The `Cameras` port in the `devices` service:** closer to device detection, but its
  adapter would need the `files` service for `ReadOnlyFileSystem` as well, which
  architecture §3 forbids.
- **Copy the camera to a temporary folder first:** reuses the folder import, but writes
  everything twice, needs the space, and the format readiness would be about the
  temporary folder.

## Consequences

- The Nix shell and every build link libgphoto2 and its libusb; a release bundles them.
- Cameras are tested in CI only through the `directory` driver. PTP and MTP themselves
  are covered by a manual hardware check before release (requirements §9): Fujifilm
  X-T2 and X-T5, and an Android phone.
- The modification time of a file on a camera is what the driver reports; PTP stores
  the camera's local time without a zone, which libgphoto2 reads as the computer's
  local time. The capture time from EXIF is not affected (CORE-005).
- On macOS, `ptpcamerad` claims a camera as soon as it is connected, so the person has
  to quit it before an import; the error message says so.
- A camera that is unplugged during a run fails each remaining file; the run ends with
  exit code 1 and the camera is not ready to be formatted.
