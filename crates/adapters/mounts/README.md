<!--
SPDX-FileCopyrightText: 2026 Framehive contributors
SPDX-License-Identifier: GPL-3.0-or-later
-->

# adapter-mounts

Layer: `adapter`, port `devices` (architecture §3)

## Purpose

Implements the `DeviceSource` port of `devices` over the file systems the desktop has
mounted, with the standard library only (DEVICES-001, ADR 0023 §2, §4). It adds no
dependency, links no native code and has no `unsafe` code.

- **Linux:** reads `/proc/self/mountinfo` (proc(5)) and considers the mounts strictly
  below `/run/media/` and `/media/`. Octal escapes such as `\040` are decoded; the
  mount source, such as `/dev/sdb1`, is kept for the eject. A mount point listed twice
  is listed once, with the later (top) mount.
- **macOS:** considers the folders directly in `/Volumes/`; symbolic links, such as
  the startup disk, are skipped.
- **Only cards:** a considered mount is a device only if `<mount point>/DCIM` is a
  folder (DCF). A missing `DCIM`, a file named `DCIM`, or one that cannot be examined
  is no device.
- **Eject:** runs `udisksctl unmount --block-device <source> --no-user-interaction`
  on Linux or `diskutil eject <mount point>` on macOS with `std::process::Command`. A
  tool that cannot be started or exits with a failure gives `EjectError::Failed` with
  its message or exit status; a device that is not listed gives
  `EjectError::NotPresent`. It never powers a device off (ADR 0023 §4).
- **Platform:** `MountsDeviceSource::new` picks the macOS way on macOS and the Linux
  way elsewhere. Both ways are constructors parameterised by paths, so the tests of
  both run on every platform against fake mount tables.
- **Not covered here:** polling and change detection (`devices::DeviceWatch`),
  mounting devices, and cameras over PTP or MTP.

## Public API

| Item | Meaning |
|---|---|
| `MountsDeviceSource` | `devices::DeviceSource` over mounts; `new()` or `Default` for this system, `linux(mountinfo, considered)` and `macos(volumes)` for given paths, `with_runner` to replace how eject commands run |
| `EjectCommand` | Program and arguments of an eject: `udisksctl(source)`, `diskutil(mount point)` |
| `run_command`, `RunCommand` | Runs an eject command as a child process; the type of a replacement for tests |

## Conformance

`tests/conformance.rs` runs `devices::testing::check_device_source` against the Linux
way with a fake `mountinfo` file and against the macOS way with a fake `Volumes`
folder, both in temporary folders, with stand-ins for `udisksctl` and `diskutil`.
`tests/listing.rs` covers the considered locations, escapes, symbolic links, unreadable
mount tables and the eject commands. Tests never run `udisksctl` or `diskutil`; the
failure paths of `run_command` use a program that does not exist and `sh`.

Because an adapter may depend only on its port's crate (architecture §3), the tests
use their own temporary folder in `tests/support/` instead of
`files::testing::TempDir`.

## Dependencies

- `devices`
