# IMPORT-005 – Source scan

- Status: draft
- Module: import
- Origin: Q4, Q6, V7, 2.7, N26
- Depends on: IMPORT-001

## Purpose

Before anything can be planned or copied, the import must know which files a source
holds. A scan finds every photo and video in a folder, wherever the camera or the user
put it, and says which files it leaves out and why.

## Behaviour

A scan takes a source folder and walks it with all its subfolders. Unlike Rapid Photo
Downloader (Q6), it does not stop at `DCIM`: the whole source is scanned, so files
copied to a card or folder by hand are found too.

Each file is classified by its extension, ignoring case:

| Role | Extensions |
|---|---|
| `Raw` | `3FR`, `ARW`, `CR2`, `CR3`, `CRW`, `DCR`, `DNG`, `FFF`, `IIQ`, `MEF`, `MOS`, `MRW`, `NEF`, `NRW`, `ORF`, `PEF`, `RAF`, `RAW`, `RW2`, `SR2`, `SRW`, `X3F` |
| `Rendered` | `JPG`, `JPEG`, `HEIF`, `HEIC`, `HIF`, `TIF`, `TIFF`, `MPO`, `AVIF`, `JXL`, `PNG` |
| `Video` | `3GP`, `AVI`, `M2T`, `M2TS`, `MOV`, `MP4`, `MPEG`, `MPG`, `MOD`, `TOD`, `MTS`, `MKV` |
| `Sidecar` | `XMP`, `THM` |

The lists are the formats of requirements section 2.7.

The scan leaves out, and lists with the reason:

- files with any other extension or without one ("unsupported type"),
- hidden files and folders, whose name starts with `.`, such as `.Trashes` and the `._`
  files macOS writes to memory cards ("hidden"),
- symbolic links, which are not followed, so a scan cannot loop or leave the source
  ("symbolic link").

A folder or file that cannot be read is listed with the reason, and the scan goes on
(N26). A source that does not exist or is not a folder fails the scan as a whole.

For each file found, the scan gives its path relative to the source, its role, its
size and its modification time. The files are listed in the byte order of their
relative paths, so the same source always gives the same list.

The destination must not lie inside the source, because a later import would find the
files it imported before. Such an import is refused before the scan starts.

## Acceptance criteria

### IMPORT-005.1 – The whole source is scanned
- **Given** a source with `DCIM/100_FUJI/DSCF0001.RAF` and `copied/holiday.jpg`
- **When** it is scanned
- **Then** both files are found

### IMPORT-005.2 – Roles come from the extension, ignoring case
- **Given** `DSCF0001.RAF`, `photo.jpeg`, `clip.MOV`, `clip.Thm` and `DSCF0001.xmp`
- **When** the source is scanned
- **Then** their roles are `Raw`, `Rendered`, `Video`, `Sidecar` and `Sidecar`

### IMPORT-005.3 – Unsupported files are listed, not imported
- **Given** a source with `DSCF0001.RAF`, `notes.txt` and `README`
- **When** it is scanned
- **Then** only `DSCF0001.RAF` is found, and `notes.txt` and `README` are listed as
  left out with the reason "unsupported type"

### IMPORT-005.4 – Hidden files and folders are left out
- **Given** a source with `DSCF0001.RAF`, `._DSCF0001.RAF` and `.Trashes/DSCF0002.RAF`
- **When** it is scanned
- **Then** only `DSCF0001.RAF` is found, and `._DSCF0001.RAF` and `.Trashes` are listed
  as left out with the reason "hidden"

### IMPORT-005.5 – Symbolic links are not followed
- **Given** a source with a symbolic link `all` that points to the source itself, and
  a symbolic link `photo.jpg` to a file
- **When** it is scanned
- **Then** the scan ends, and `all` and `photo.jpg` are listed as left out with the
  reason "symbolic link"

### IMPORT-005.6 – An unreadable folder does not stop the scan
- **Given** a source with a folder `locked` that cannot be read, and `DSCF0001.RAF`
- **When** it is scanned
- **Then** `DSCF0001.RAF` is found, and `locked` is listed with the reason it could not
  be read

### IMPORT-005.7 – The order is stable
- **Given** a source with `b/2.jpg`, `a/1.jpg` and `B.jpg`
- **When** it is scanned
- **Then** the files are listed as `B.jpg`, `a/1.jpg`, `b/2.jpg`, every time

### IMPORT-005.8 – Size and modification time are given
- **Given** `DSCF0001.RAF` with 1,024 bytes, modified at 2026-10-17 14:05:09 UTC
- **When** the source is scanned
- **Then** the file is found with the size 1,024 bytes and that modification time

### IMPORT-005.9 – An empty source is not an error
- **Given** a source folder without files
- **When** it is scanned
- **Then** the scan succeeds and finds no files

### IMPORT-005.10 – A missing source fails the scan
- **Given** a source path that does not exist, or that is a file
- **When** it is scanned
- **Then** the scan fails with an error that names the path

### IMPORT-005.11 – A destination inside the source is refused
- **Given** the source `/media/card` and the destination `/media/card/library`
- **When** the import is started
- **Then** it is refused before the scan, with an error that names both paths, and no
  file is copied

## Out of scope

- Detecting cards, cameras and phones (Q1, Q2): the device spec; this spec scans a
  folder.
- Configurable scanned folders and ignore patterns (Q6), as part of a profile (N8).
- Grouping files into assets (LIBRARY-001); until it is implemented, each file is its
  own asset.
- The capture time of a file: CORE-005.
- Files already imported, and name conflicts: IMPORT-003.
- Deduplicating two card slots (Q8), and audio and proxy files (V7, G2): later specs.

## Open questions

1. Sidecars without grouping: as long as each file is its own asset, a sidecar is named
   from its own modification time, not from its photo's capture time, and can land in
   another folder than its photo (IMPORT-001, "Assets"). Recommendation: until
   LIBRARY-001 is implemented, the scan finds sidecars but leaves them out, with the
   reason "sidecar, needs grouping", so none is separated from its photo. Fujifilm
   cameras write no sidecars, so milestone 1 loses nothing for the reference cameras.

## History

| Date | Change |
|---|---|
| 2026-10-31 | Initial draft (milestone 1) |
