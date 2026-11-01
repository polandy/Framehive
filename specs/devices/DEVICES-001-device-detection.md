# DEVICES-001 – Device detection

- Status: implemented
- Module: devices
- Origin: Q1, A3, G14, N22
- Depends on: –

## Purpose

To import a memory card without typing its path, Framehive must know which cards are
inserted, notice a card that is inserted or pulled while it runs, and eject a card after
the import. Decisions and alternatives are in ADR 0023.

## Behaviour

A **device** is a removable storage medium as it is offered for import (glossary). It
has a **name**, the last component of its mount point (for example `EOS_DIGITAL`), and
its **mount point**, which identifies it while it is mounted.

The `devices` service defines the port `DeviceSource`. A device source lists the devices
that are present, sorted by mount point, and ejects one of them.

The `mounts` adapter finds devices among the mounted file systems:

| Platform | Mounts considered |
|---|---|
| Linux | Every mount in `/proc/self/mountinfo` whose mount point is below `/run/media/` or `/media/` |
| macOS | Every folder directly in `/Volumes/`; symbolic links are skipped |

A considered mount is a device only if it has a folder named `DCIM` at its top level
(DCF). A mount whose `DCIM` folder is missing, is a file, or cannot be examined is not a
device.

A **device watch** turns successive lists into changes. Each poll reads the list from
the device source and compares it with the list of the last successful poll; before the
first poll that list is empty. It reports the devices that were removed, then the
devices that were added, each sorted by mount point.

Ejecting a device makes it safe to pull:

| Platform | Eject |
|---|---|
| Linux | `udisksctl unmount --block-device <device> --no-user-interaction`, where `<device>` is the mount source from `mountinfo` |
| macOS | `diskutil eject <mount point>` |

## Acceptance criteria

### DEVICES-001.1 – A card is listed
- **Given** a mounted file system below a considered location with a folder `DCIM` at
  its top level
- **When** the device source lists the devices
- **Then** it lists a device whose mount point is that mount point and whose name is the
  last component of the mount point.

### DEVICES-001.2 – A drive without `DCIM` is not listed
- **Given** a mounted file system below a considered location that has no `DCIM`, or
  has a file named `DCIM`
- **When** the device source lists the devices
- **Then** it does not list that file system.

### DEVICES-001.3 – The list is sorted
- **Given** several cards are mounted
- **When** the device source lists the devices
- **Then** they are sorted by mount point, in byte order of the path.

### DEVICES-001.4 – Only removable locations on Linux
- **Given** `mountinfo` lists mounts at `/`, `/home`, `/mnt/nas` and
  `/run/media/anna/EOS_DIGITAL`, each with a folder `DCIM`
- **When** the mounts adapter lists the devices on Linux
- **Then** it lists only `/run/media/anna/EOS_DIGITAL`; a mount below `/media/` is
  listed in the same way.

### DEVICES-001.5 – Escaped mount points on Linux
- **Given** `mountinfo` lists the mount point `/run/media/anna/NO\040NAME`
- **When** the mounts adapter lists the devices on Linux
- **Then** the device's mount point is `/run/media/anna/NO NAME` and its name is
  `NO NAME`.

### DEVICES-001.6 – Volumes on macOS
- **Given** `/Volumes/` holds a folder `Untitled` with `DCIM` and a symbolic link
  `Macintosh HD` to `/`
- **When** the mounts adapter lists the devices on macOS
- **Then** it lists `Untitled` only.

### DEVICES-001.7 – The mount table cannot be read
- **Given** the device source cannot read its list of mounts
- **When** it lists the devices
- **Then** it fails with an error that names what could not be read.

### DEVICES-001.8 – The first poll reports every device as added
- **Given** a new device watch and a device source that lists two cards
- **When** the watch polls
- **Then** it reports both cards as added and none as removed.

### DEVICES-001.9 – Later polls report changes only
- **Given** a device watch whose last poll saw cards A and B
- **When** the watch polls and the source lists B and C
- **Then** it reports A as removed and C as added, and nothing about B; a poll with an
  unchanged list reports nothing.

### DEVICES-001.10 – A failed poll changes nothing
- **Given** a device watch whose last poll saw card A
- **When** a poll fails because the source cannot list the devices, and a later poll
  lists no device
- **Then** the failed poll returns the source's error and reports no change, and the
  later poll reports A as removed.

### DEVICES-001.11 – Ejecting a card
- **Given** a listed card
- **When** it is ejected and the eject succeeds
- **Then** the device source no longer lists it.

### DEVICES-001.12 – The eject command
- **Given** a card at `/run/media/anna/EOS_DIGITAL` mounted from `/dev/sdb1` on Linux,
  or at `/Volumes/Untitled` on macOS
- **When** the mounts adapter ejects it
- **Then** it runs `udisksctl unmount --block-device /dev/sdb1 --no-user-interaction`
  on Linux, or `diskutil eject /Volumes/Untitled` on macOS.

### DEVICES-001.13 – A failed eject
- **Given** a listed card
- **When** the eject tool cannot be started, or exits with a failure
- **Then** the eject fails with an error that names the device and carries the tool's
  message or exit status, and the card is still listed.

### DEVICES-001.14 – Ejecting a device that is not listed
- **Given** a device that the device source does not list, for example because it was
  pulled
- **When** it is ejected
- **Then** the eject fails with an error that says the device is not present.

## Out of scope

- Mounting devices that the system did not mount (A1).
- Cameras and phones over PTP or MTP (Q2), iOS devices (Q3).
- Free space and capacity of a device (Q7).
- Remembering devices (Q9) and binding profiles to devices (N8).
- Waiting for cards and asking the user: the guided import (CLI-002).

## Open questions

–

## History

| Date | Change |
|---|---|
| 2026-11-01 | Created (ADR 0023) |
| 2026-11-01 | Implemented in `devices` and `adapter-mounts` |
