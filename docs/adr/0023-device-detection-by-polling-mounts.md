# 0023 – Device detection by polling mounts

- Status: accepted
- Date: 2026-11-01

## Context

The planned guided import waits for a memory card, imports it and offers to eject it.
That needs three things the code base does not have: a list of the devices that are
offered for import, a way to notice a card that is inserted or pulled while Framehive
runs (Q1), and a way to eject a card after the import (A3). Only removable media count,
never every local drive (G14). Device access must stay behind a trait, so that Linux,
macOS and later cameras can be added (N22, architecture §5: `devices::DeviceSource`).

The desktop already mounts a memory card when it is inserted: udisks2 on Linux below
`/run/media/<user>/` or `/media/<user>/`, Disk Arbitration on macOS below `/Volumes/`.
Mounting unmounted devices ourselves (A1) is not needed for the first guided import.

The direct ways to hear about new devices are D-Bus signals from udisks2 (the `zbus`
crate), `libudev` (native), and Disk Arbitration on macOS (FFI to a system framework).
Each adds a dependency tree or native code (ADR 0009) for a feature that a person uses
by inserting a card and waiting a second.

## Decision

### 1. A `devices` service with the port `DeviceSource`

A new service crate `crates/services/devices` (package `devices`, layer `service`) holds:

- `Device`: a device as it is offered for import, with its name and its mount point.
  Its mount point identifies it while it is mounted.
- The port `DeviceSource`: list the devices that are present, and eject one.
- `DeviceWatch`: compares the list of one poll with the list of the poll before and
  reports which devices were added and which were removed. It does not wait; the caller
  decides when to poll, so tests poll by hand and never sleep (ADR 0006).
- `testing`: `MemoryDeviceSource` and the conformance suite of the port, which every
  adapter runs (architecture §5).

The service depends on `domain` and `thiserror` only. The behaviour is specified in
DEVICES-001; the spec prefix `DEVICES` is added to `specs/README.md`.

### 2. A `mounts` adapter that polls with the standard library

`adapter-mounts` implements `DeviceSource` with `std` only:

- **Linux:** it reads `/proc/self/mountinfo` and considers the mount points below
  `/run/media/` and `/media/`, where udisks2 and the common automounters put removable
  media. Other mounts, such as `/`, `/home` or a network share in `/mnt/`, are never
  offered.
- **macOS:** it considers the folders directly in `/Volumes/`. The startup disk appears
  there as a symbolic link to `/` and is skipped.
- **Only cards (DCF):** a considered mount is a device only if it has a folder `DCIM` at
  its top level, the folder that the Design rule for Camera File system (DCF) requires
  on every camera card. A USB stick with documents is not offered.

Both ways of finding mounts are plain functions over text and folders, so the tests of
both run on every platform with a fake `mountinfo` file and a temporary folder.

### 3. Polling, not events

The caller polls the source. The guided import polls once a second; the interval is the
app's choice and not part of the service. Listing mounts and checking one folder per
mount is cheap.

### 4. Eject through the system's tools

The adapter ejects by running the tool the system provides for it, as a child process:

- **Linux:** `udisksctl unmount --block-device <device> --no-user-interaction`, with the
  device from the mount's source in `mountinfo`. This is what the file manager does on
  "unmount"; the card can then be pulled safely, because a successful unmount has
  written all data. The adapter does not power the device off, because powering off a
  built-in card reader can disable it until the next boot.
- **macOS:** `diskutil eject <mount point>`.

If the tool is missing or fails, for example because a file on the card is still open,
the eject fails with the tool's message, and the card stays listed.

### 5. No new dependency

No crate is added, no native code is linked, and no `unsafe` code is written. Child
processes are started with `std::process::Command`.

## Alternatives

- **udisks2 over D-Bus with `zbus`:** events instead of polling, the removable flag of
  the drive, and unmount and power-off in-process. It adds an async D-Bus stack of about
  twenty crates for a one-second gain, and does not help on macOS.
- **`libudev`:** native code and an ADR for a native library, Linux only, and still no
  mount points.
- **Disk Arbitration on macOS:** FFI to a framework and `unsafe` code in an FFI adapter.
  Worth it once Framehive mounts devices itself (A1) or needs events in a GUI.
- **Every mount with a `DCIM` folder:** simpler, but would offer a network share or a
  backup disk that holds a copy of a card, against G14.
- **Watching `/proc/self/mountinfo` with `poll(2)`:** the kernel signals a changed mount
  table, but waiting for it needs `libc` and `unsafe` code, and it does not exist on
  macOS.

## Consequences

- A card appears up to one poll interval after it is mounted.
- A card that is mounted somewhere else, for example by hand below `/mnt/`, is not
  offered; `framehive import <source> <destination>` still imports any folder.
- A card that the system does not mount, and cameras over PTP or MTP (Q2), are not
  offered yet. They become further `DeviceSource` adapters; gphoto2 needs its own ADR
  (native library).
- Ejecting on Linux needs udisks2, which every common desktop installs; without it the
  eject fails with a clear message.
- If events are needed later, an event-based adapter can replace polling behind the
  same port; `DeviceWatch` then stays the place that turns lists into changes.
