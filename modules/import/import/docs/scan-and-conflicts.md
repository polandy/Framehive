<!--
SPDX-FileCopyrightText: 2026 Framehive contributors
SPDX-License-Identifier: GPL-3.0-or-later
-->

# Source scan and conflict resolution

How the `import` crate scans a source and resolves name conflicts. The crate's
[README](../README.md) has its purpose, public API and boundaries.

The source scan (IMPORT-005) walks the source folder and lists each supported file
with its role, size and modification time, and each file or folder it leaves out with
the reason: unsupported type, hidden, symbolic link, sidecar until grouping, or cannot
be read. `scan_camera` scans a camera or phone (IMPORT-014): only the folder `DCIM` at
the top of each storage, with the same rules, giving paths that start with the storage
folder. A camera without any storage folder, such as a locked phone, is
`ScanError::NoStorage`, not an empty source; `CAMERA_FORMAT_READINESS_NOTE` is the note
its format readiness carries.

`TargetResolver` resolves name conflicts and files already imported
(IMPORT-003) against the destination and the earlier files of the plan, comparing
names by their canonical caseless key, without regard to case and Unicode
normalization form (ADR 0026); a destination may hold several names with one key,
and each is compared. Content is compared by size and checksum, under the conflict
policy `add_suffix`, `skip` or `fail`. A file at the destination where a folder of the
target path should be fails the file under every policy, and the folders of a video
destination inside the import destination count as taken for photos (IMPORT-017).

A file that gets an XMP sidecar (IMPORT-019) passes its `SidecarNaming` in the
`TargetRequest`. A candidate name is then free only when the sidecar's name is free as
well: not claimed by an earlier file or sidecar of the plan, not reserved by another
job, and nothing at the destination, so a leftover sidecar is never merged with. The
file claims both names, and a suffix goes to both. A file with the same content still
counts as already imported, whatever is next to it.
