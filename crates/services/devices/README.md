<!--
SPDX-FileCopyrightText: 2026 Framehive contributors
SPDX-License-Identifier: GPL-3.0-or-later
-->

# devices

Layer: `service` (architecture §2, §3)

## Purpose

The devices service holds the port through which Framehive finds the devices offered
for import and ejects them, and the device watch that notices a card that is inserted
or pulled (DEVICES-001, ADR 0023).

Boundaries:

- **Port only.** The service never reads mounts or starts processes; `adapter-mounts`
  implements `DeviceSource`.
- **A device is a card or a camera.** A device source lists only cards: mounts with a
  `DCIM` folder at their top level (DCF), sorted by mount point in byte order of the
  path. The mount point identifies a card; its name is the last component of the mount
  point. A camera comes from the `Cameras` port of the `files` service and is identified
  by its model and port; a camera plugged in again is a new device. Cards sort before
  cameras (DEVICES-001.15, 16).
- **Polling, no waiting.** `DeviceWatch::poll` lists the cards and the cameras once and
  compares the list with the last poll: removed devices first, then added ones, each
  sorted. The first poll reports every device as added. The two kinds are listed
  independently: a kind that fails keeps its last devices and adds a `PollError`, the
  other is still reported (DEVICES-001.19, 20). The caller decides when to poll, so tests
  never sleep (ADR 0023 §3).
- **Eject.** A device that is not listed, and every camera, fails with
  `EjectError::NotPresent`; a failed
  eject names the device and the tool's message, and the device stays listed.
- **Not covered here:** mounting devices, the camera adapter itself, and waiting for a
  card in the guided import (CLI-002).

## Public API

| Item | Meaning |
|---|---|
| `Device`, `DeviceKind` | A card (mount point) or a camera (port) with its name; cards before cameras |
| `DeviceSource` | Port that lists the present devices and ejects one |
| `ListError` | Listing failed; names what could not be read (DEVICES-001.7) |
| `EjectError` | Ejecting failed: `NotPresent`, `Failed` with the tool's message, or `List` |
| `DeviceWatch`, `DeviceChanges`, `DevicePoll`, `PollError` | Polls cards and cameras and turns successive lists into removed and added devices, with one error per kind that failed (DEVICES-001.8–10, 17–20) |
| `testing::MemoryDeviceSource` | `DeviceSource` in memory, with mounts with or without `DCIM` and injectable listing and eject failures |
| `testing::check_device_source`, `testing::DeviceSourceUnderTest`, `testing::DcimFolder` | Conformance suite of the `DeviceSource` port |

## Dependencies

- `thiserror` (ADR 0016)
- `files` service, for the `Cameras` port (ADR 0023 amendment)
