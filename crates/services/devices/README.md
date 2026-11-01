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
- **Devices are cards.** A device source lists only mounts with a `DCIM` folder at
  their top level (DCF), sorted by mount point in byte order of the path. The mount
  point identifies a device; its name is the last component of the mount point.
- **Polling, no waiting.** `DeviceWatch::poll` lists the source once and compares the
  list with the last successful poll: removed devices first, then added ones, each
  sorted. The first poll reports every device as added; a failed poll returns the
  source's error and keeps the last list. The caller decides when to poll, so tests
  never sleep (ADR 0023 §3).
- **Eject.** A device that is not listed fails with `EjectError::NotPresent`; a failed
  eject names the device and the tool's message, and the device stays listed.
- **Not covered here:** mounting devices, cameras over PTP or MTP, and waiting for a
  card in the guided import (CLI-002).

## Public API

| Item | Meaning |
|---|---|
| `Device` | A device with its name and mount point; ordered by the bytes of the mount point |
| `DeviceSource` | Port that lists the present devices and ejects one |
| `ListError` | Listing failed; names what could not be read (DEVICES-001.7) |
| `EjectError` | Ejecting failed: `NotPresent`, `Failed` with the tool's message, or `List` |
| `DeviceWatch`, `DeviceChanges` | Turns successive lists into removed and added devices (DEVICES-001.8–10) |
| `testing::MemoryDeviceSource` | `DeviceSource` in memory, with mounts with or without `DCIM` and injectable listing and eject failures |
| `testing::check_device_source`, `testing::DeviceSourceUnderTest`, `testing::DcimFolder` | Conformance suite of the `DeviceSource` port |

## Dependencies

- `thiserror` (ADR 0016)
