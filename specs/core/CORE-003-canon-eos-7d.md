# CORE-003 – Canon EOS 7D support (CR2)

- Status: draft
- Module: core (metadata and previews services)
- Origin: C1–C6, 2.7, 2.8, V7
- Depends on: LIBRARY-001 (grouping)

## Purpose

The Canon EOS 7D is the project owner's earlier camera. Its files make up a large part
of the existing archive. They must work as fully as the current cameras' files:
import, thumbnails, loupe, 100% zoom, metadata and filtering.

## Behaviour

**Reference set.** It is published in the `framehive-testdata` repository and pinned
in `testdata.lock` as `canon/eos-7d/` (ADR 0017). It uses the project owner's own
photos where available, cleaned and released as CC0 like the Fujifilm set (CORE-001),
and raw.pixls.us samples otherwise.

| Files |
|---|
| CR2 in RAW (18 MP), M-RAW and S-RAW |
| CR2 + JPG pair (`IMG_1234`) |
| CR2 + JPG pair shot in Adobe RGB (`_MG_1234`) |
| Video `MVI_1234.MOV` with `MVI_1234.THM` |

**File types.**

| Kind | Extension | Notes |
|---|---|---|
| RAW | `.CR2` | TIFF-based; RAW, M-RAW and S-RAW sizes. M-RAW and S-RAW store YCbCr data, not a Bayer mosaic |
| Rendered | `.JPG` | |
| Video | `.MOV` | H.264 |
| Video thumbnail | `.THM` | Small JPEG with the video's EXIF data |

**Reading CR2.** Our own TIFF/EXIF reader in the metadata service reads the CR2
structure (ADR 0009, ADR 0012). It is the same reader that DNG uses (CORE-004). It
covers IFD0 with the embedded full-size JPEG, the EXIF IFD and the Canon maker notes.
No native library is needed for metadata, thumbnails or the normal preview.

**Metadata.**

| Field | Source |
|---|---|
| Capture time, sub-seconds | EXIF `DateTimeOriginal`, `SubSecTimeOriginal` |
| Time-zone offset | Not written by the camera; from the camera time correction (N15), otherwise unknown |
| Make, model | EXIF |
| Serial number | Canon maker note `SerialNumber` |
| Lens | Canon maker note `LensModel`; if missing, the `LensType` ID resolved through our lens table |
| Focal length, aperture, shutter speed, ISO, exposure compensation | EXIF |
| Picture style, drive mode, image quality (RAW / M-RAW / S-RAW) | Canon maker notes |

The picture style is shown with the camera's names (*Standard*, *Portrait*,
*Landscape*, *Neutral*, *Faithful*, *Monochrome*, *User Def. 1–3*) and is filterable,
like the Fujifilm film simulation (CORE-001).

**Previews.** Thumbnails and the loupe preview come from the embedded JPEG. For 100%
zoom, the RAW is decoded via the `PreviewDecoder` port (LibRaw, ADR 0012) as a
background job.

**Grouping.** CR2 + JPG pairs group under LIBRARY-001. The camera's `_MG_` prefix in
Adobe RGB mode is part of the base name, so `_MG_1234.CR2` and `_MG_1234.JPG` group,
and `IMG_1234.JPG` does not join them. A `.THM` file belongs to the video with the same
base name as a `Sidecar` member. It is imported, moved and deleted with the video and
never shown as its own asset. Its EXIF fills gaps in the video's metadata.

**Damaged files** are handled as in CORE-001: readable metadata is kept, and the asset
is marked "damaged" with the reason.

## Acceptance criteria

### CORE-003.1 – Metadata from CR2
- **Given** the reference file `canon/eos-7d/raw.CR2`
- **When** its metadata is read
- **Then** make, model, serial number, capture time, lens, exposure and picture style
  match its `.expected.yaml`

### CORE-003.2 – M-RAW and S-RAW
- **Given** the reference files `canon/eos-7d/{mraw,sraw}.CR2`
- **When** their metadata is read, a thumbnail is generated and they are fully decoded
- **Then** the metadata match their `.expected.yaml`, the image quality is reported as
  M-RAW or S-RAW, and the decoded size is 3888×2592 or 2592×1728

### CORE-003.3 – Lens from the lens table
- **Given** a CR2 whose maker notes have a `LensType` ID but no `LensModel`
- **When** its metadata is read
- **Then** the lens name comes from the lens table, and an unknown ID is shown as
  "Unknown lens (ID <n>)"

### CORE-003.4 – No time-zone offset
- **Given** a 7D file and no camera time correction for its serial number
- **When** its metadata is read
- **Then** the capture time has no offset, and it is displayed and sorted as local time
  without conversion

### CORE-003.5 – Thumbnail without RAW decoding
- **Given** any CR2 from the reference set
- **When** a thumbnail is generated
- **Then** it is produced from the embedded JPEG, and the RAW decoder is not called

### CORE-003.6 – Full decode
- **Given** `canon/eos-7d/raw.CR2`
- **When** it is decoded at full resolution
- **Then** the output is 5184×3456, and its hash matches the stored reference hash for
  the pinned decoder version and configuration

### CORE-003.7 – Adobe RGB pair
- **Given** a folder with `_MG_1234.CR2`, `_MG_1234.JPG` and `IMG_1234.JPG`
- **When** the library indexes the folder
- **Then** there are two assets: `_MG_1234` with roles `Raw` and `Rendered`, and
  `IMG_1234` with role `Rendered`

### CORE-003.8 – Video with THM
- **Given** a card with `MVI_1234.MOV` and `MVI_1234.THM`
- **When** it is imported and the target folder is indexed
- **Then** both files are at the target with the same new base name, and there is one
  asset with roles `Video` and `Sidecar`

### CORE-003.9 – Filter by picture style
- **Given** a library with 7D assets shot in *Standard* and *Monochrome*
- **When** the user filters by *Monochrome*
- **Then** only the *Monochrome* assets are shown

## Out of scope

- Developing CR2 files: later editor phase.
- Other Canon models: supported via LibRaw and raw.pixls.us samples, without their own
  reference set. CR3 (newer Canon cameras) is a separate container format.

## Open questions

- Which lens IDs does the lens table need? It starts with the lenses in the owner's
  7D photos and the IDs that exiftool documents.

## History

| Date | Change |
|---|---|
| 2026-10-18 | Initial draft |
| 2026-10-18 | Reference set in `framehive-testdata` (ADR 0017) |
