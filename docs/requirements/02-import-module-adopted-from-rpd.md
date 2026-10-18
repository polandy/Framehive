# 2. Import module: adopted from RPD

Part of the [requirements](README.md).

## 2.1 Sources

| ID | Requirement | Priority |
|---|---|---|
| Q1 | Automatic detection of memory cards, USB drives, and cameras, even while the program is running [RPD] | M |
| Q2 | Cameras/smartphones via PTP/MTP (libgphoto2) [RPD] | M |
| Q3 | iOS devices (libimobiledevice) [RPD] | K |
| Q4 | Any folder as a source ("This Computer") [RPD] | M |
| Q5 | Scan and import multiple sources in parallel [RPD] | M |
| Q6 | Scanned folders configurable (default: DCIM, PRIVATE, MP_ROOT), ignore patterns (regex) [RPD] | M |
| Q7 | Show storage usage per device; hovering over a device highlights its thumbnails [RPD] | S |
| Q8 | Cameras with two card slots: deduplicate files across both cards, mark the slot [RPD] | S |
| Q9 | Remember devices: "always import" / "always ignore" [RPD] | S |

## 2.2 Preview & Selection

| ID | Requirement | Priority |
|---|---|---|
| V1 | Thumbnail grid, generated asynchronously and in parallel, thumbnail cache on disk [RPD] | M |
| V2 | Thumbnails for RAW, TIFF, HEIF, video [RPD] | M |
| V3 | Already-imported files shown dimmed and deselected by default; hovering shows where they were imported to [RPD] | M |
| V4 | Filter "new only / all", sorting by time, name, type, device, selection status [RPD] | M |
| V5 | Selection via checkbox, rubber-band selection, "all photos/videos", per device [RPD] | M |
| V6 | **Timeline**: grouping by time gaps between shots (5 min – 24 h, slider); selecting in the timeline filters the grid [RPD] | M |
| V7 | Associated audio/sidecar files (WAV, XMP, THM, LRV) are carried along [RPD] | S |

## 2.3 Renaming & Folder Structure

| ID | Requirement | Priority |
|---|---|---|
| R1 | File names built from components: capture date/time (various formats), import time, original name, image number, free text [RPD] | M |
| R2 | Photo metadata: aperture, ISO, shutter speed, focal length, camera make/model, serial number, shutter count, owner [RPD] | S |
| R3 | Video metadata: codec, resolution, length, FPS [RPD] | S |
| R4 | Sequences: "imported today", persistent counter, session counter, letter sequence (a…zz), 1–6 digits [RPD] | M |
| R5 | RAW + JPEG of the same image get the same sequence number [RPD] | M |
| R6 | Subfolder structure from the same components (e.g. `YYYY/YYYY-MM-DD`) [RPD] | M |
| R7 | Separate destinations for photos and videos [RPD] | M |
| R8 | Compatibility mode: strip characters invalid on Windows/macOS [RPD] | M |
| R9 | **Job codes**: free-text labels for groups of files, usable in file and folder names; assigned before import or prompted for at start; list is remembered [RPD] | S |
| R10 | Preview of the resulting folder structure before import [RPD] | M |

## 2.4 Import & Backup

| ID | Requirement | Priority |
|---|---|---|
| I1 | Block import if there isn't enough space at the destination [RPD] | M |
| I2 | Simultaneous backup to multiple destinations, same folder structure and file names [RPD] | M |
| I3 | Automatically detect backup drives (marker folder) and fixed network paths [RPD] | S |
| I4 | Separate backup destinations for photos and videos [RPD] | S |
| I5 | Conflicts: skip or append a unique suffix (destination); overwrite or skip (backup) [RPD] | M |
| I6 | Timezone/DST tolerance when recognizing files on FAT32 (15/30/60 min) [RPD] | S |

## 2.5 Automation

| ID | Requirement | Priority |
|---|---|---|
| A1 | Automatically mount unmounted devices [RPD] | S |
| A2 | Start import on program launch or when a device is plugged in [RPD] | S |
| A3 | Unmount device after import [RPD] | S |
| A4 | Quit the program after import (optionally only if there were no errors) [RPD] | K |

## 2.6 Other

| ID | Requirement | Priority |
|---|---|---|
| X1 | Command line: set source, destination, backup; reset settings; clear cache [RPD] | S |
| X2 | Multilingual [RPD] | S |
| X3 | HiDPI, native Wayland [RPD] | M |
| X4 | Thumbnail cache with size limit and expiry; generate Freedesktop thumbnails for file managers [RPD] | S |
| X5 | Configurable warnings (missing backup, unknown file types, filtered files excluded from import) [RPD] | S |
| X6 | "Report a problem": bundle logs and configuration into an archive [RPD] | K |

## 2.7 Formats

- RAW: 3FR, ARW, CR2, CR3, CRW, DCR, DNG, FFF, IIQ, MEF, MOS, MRW, NEF, NRW, ORF, PEF,
  RAF, RAW, RW2, SR2, SRW, X3F
- Photo: JPEG, HEIF/HEIC (including Fujifilm `.HIF`), TIFF, MPO, **[NEW]** AVIF, JPEG XL, PNG
- Video: 3GP, AVI, M2T, M2TS, MOV, MP4, MPEG, MPG, MOD, TOD, MTS, **[NEW]** MKV,
  Insta360/GoPro formats

## 2.8 Reference cameras and formats [NEW]

The files of the reference cameras and DNG are a release criterion: if they don't work
fully, the release is blocked.

### Fujifilm X-T2 and X-T5

The project owner shoots with a Fujifilm X-T2 (X-Trans III, 24 MP) and an X-T5
(X-Trans V, 40 MP). Binding behaviour: `specs/core/CORE-001-fujifilm-raf.md`.

| ID | Requirement | Priority |
|---|---|---|
| F1 | Read RAF files from both cameras in every compression mode the camera offers (uncompressed, lossless compressed; X-T5 also lossy compressed) | M |
| F2 | Metadata from RAF: EXIF (capture time incl. sub-seconds and time-zone offset if present, camera, lens, exposure, GPS) and camera serial number | M |
| F3 | Fujifilm maker notes: film simulation, dynamic range, grain, white balance, drive mode, image number, shown in the info panel and filterable (film simulation, B14) | S |
| F4 | Thumbnails and loupe preview from the embedded JPEG without decoding the RAW; full RAW decode (X-Trans demosaicing) for 100% zoom and comparison (ADR 0012) | M |
| F5 | X-T5 HEIF (`.HIF`) read, previewed and paired with RAF (B23) | M |
| F6 | Both cameras' dual-slot modes work: "backup" (same files on both cards, Q8) and "RAW / JPEG separation" (RAF on one card, JPEG/HEIF on the other, paired anyway, B24) | M |
| F7 | Real sample files from both cameras are part of the test data (9.3) | M |

### Canon EOS 7D

The project owner's earlier camera (APS-C, 18 MP, Bayer sensor). Its files make up a
large part of the existing archive. Binding behaviour:
`specs/core/CORE-003-canon-eos-7d.md`.

| ID | Requirement | Priority |
|---|---|---|
| C1 | Read CR2 files in every size the camera offers: RAW, M-RAW and S-RAW | M |
| C2 | Metadata from CR2: EXIF (capture time incl. sub-seconds, camera, exposure) and, from the Canon maker notes, lens, serial number and picture style. The 7D writes no time-zone offset; it comes from the camera time correction (N15) | M |
| C3 | Thumbnails and loupe preview from the embedded JPEG; full RAW decode for 100% zoom (ADR 0012) | M |
| C4 | CR2 + JPG pairs group into one asset (B23), including the `_MG_` prefix the camera uses in Adobe RGB mode | M |
| C5 | Videos (`MVI_*.MOV`) keep their `.THM` file: it is carried along on import (V7) and belongs to the video's asset | M |
| C6 | Real sample files are part of the test data (9.3) | M |

### DNG

Adobe's open RAW format (DNG Specification 1.0–1.7). The project owner's DNGs come from
the Adobe DNG Converter and from drones and action cameras. Binding behaviour:
`specs/core/CORE-004-dng.md`, ADR 0015.

| ID | Requirement | Priority |
|---|---|---|
| D1 | Read DNG files of every DNG version up to 1.7: mosaic and linear (demosaiced) data; uncompressed, lossless and lossy compressed | M |
| D2 | DNG from the Adobe DNG Converter, including files without an embedded preview and files with embedded original RAW data | M |
| D3 | DNG from drones (e.g. DJI), including GPS altitude and the manufacturer's XMP namespaces, which are preserved unchanged | M |
| D4 | GoPro GPR (VC-5 compressed DNG): metadata and preview; full decode if the RAW decoder supports it (G2) | S |
| D5 | Embedded XMP in a DNG is read. Metadata is written to the sidecar; an option writes it into the DNG as well, so that Lightroom sees it (ADR 0015) | M |
| D6 | Thumbnail from the largest embedded preview; if there is none, from a RAW decode in the background | M |
| D7 | Real sample files are part of the test data (9.3) | M |
