# CORE-001 – Fujifilm RAF support (X-T2, X-T5)

- Status: draft
- Module: core (metadata and previews services)
- Origin: F1–F7, 2.7, 2.8
- Depends on: –

## Purpose

The Fujifilm X-T2 and X-T5 are the project owner's cameras and Framehive's reference
cameras. Their files must work fully: import, thumbnails, loupe, 100% zoom, metadata
and filtering.

## Behaviour

**Reference set.** It is published in the `framehive-testdata` repository and pinned
in `testdata.lock` as `fujifilm/x-t2/` and `fujifilm/x-t5/` (ADR 0017), one set per
camera. It consists of the project owner's own photos, cleaned before they are
published: GPS removed, serial numbers replaced by a test value, no recognisable people. They are not CC0: the owner keeps the copyright, and the
Framehive Test Data Licence permits redistribution and use only for testing Framehive.
raw.pixls.us samples complement it.

| Camera | Wanted files | Published |
|---|---|---|
| X-T2 | uncompressed and lossless compressed RAF; RAF + JPG pair; one shot with *Classic Chrome* | `x-t2-stone-villa`: lossless RAF + JPG pair (*Provia*), with darktable sidecars |
| X-T5 | uncompressed, lossless and lossy compressed RAF; RAF + JPG and RAF + HIF pairs; one shot with *Nostalgic Neg.*; a dual-slot "RAW / JPEG separation" pair | `x-t5-mountain-lake`: lossless RAF + JPG pair (*Velvia*), with darktable sidecars |

By owner decision of 2026-11-10 there will be no further reference files for the X-T2
and X-T5. The wanted files that are not published (X-T2: uncompressed RAF, *Classic
Chrome*; X-T5: uncompressed and lossy RAF, RAF + HIF pair, *Nostalgic Neg.*, dual-slot
"RAW / JPEG separation" pair) have no fixture. CORE-001.2, CORE-001.3 and CORE-001.6
are not covered by reference tests for those cases.

Because the serial numbers are removed, tests that need one use the value written by
the cleaning script (`fujifilm-x-t2/clean.yaml` and `fujifilm-x-t5/clean.yaml` in
`framehive-testdata` document every change).

**File types.**

| Camera | Sensor | RAW | Rendered | RAF compression |
|---|---|---|---|---|
| X-T2 | X-Trans III, 24 MP | `.RAF` | `.JPG` | uncompressed, lossless compressed |
| X-T5 | X-Trans V, 40 MP | `.RAF` | `.JPG`, `.HIF` (HEIF) | uncompressed, lossless compressed, compressed (lossy) |

**Reading RAF.** Our own code reads the RAF container in the metadata service (ADR
0009, ADR 0012). It covers the header with the camera model and the offsets to the
embedded JPEG and the RAW data, the embedded JPEG with its EXIF, and the Fujifilm
maker notes. No native library is needed for metadata, thumbnails or the normal
preview.

**Metadata.** The following values are read:

| Field | Source |
|---|---|
| Capture time, sub-seconds, time-zone offset (if written) | EXIF `DateTimeOriginal`, `SubSecTimeOriginal`, `OffsetTimeOriginal` |
| Make, model, serial number | EXIF, Fujifilm maker note |
| Lens, focal length, aperture, shutter speed, ISO, exposure compensation | EXIF |
| GPS (if set via the smartphone app) | EXIF GPS IFD |
| Film simulation, dynamic range, grain effect, white balance, drive mode, image number | Fujifilm maker note |

The film simulation is shown with the camera's names (e.g. *Classic Chrome*,
*Classic Neg.*, *Acros+R*, *Nostalgic Neg.*). It is filterable in the filter bar.

**Previews.** Thumbnails and the loupe preview come from the embedded JPEG. For 100%
zoom and comparison at full resolution, the RAW is decoded via the `PreviewDecoder`
port (LibRaw, ADR 0012) as a background job. The embedded preview is shown until the
decode is ready.

**Unknown or damaged files.** A damaged RAF never crashes the application. Metadata
that can be read is kept. The asset gets a "damaged" marker with the reason.

## Acceptance criteria

### CORE-001.1 – Metadata from X-T2 RAF
- **Given** the reference file `fujifilm/x-t2/x-t2-stone-villa.RAF` (lossless compressed)
- **When** its metadata is read
- **Then** make, model, serial number, capture time, lens and exposure match the values
  in its `.expected.yaml`

### CORE-001.2 – Metadata from X-T5 RAF in every compression mode
- **Given** the reference files `fujifilm/x-t5/{uncompressed,lossless,lossy}.RAF`
- **When** their metadata is read
- **Then** all values match their `.expected.yaml`

### CORE-001.3 – Film simulation
- **Given** reference files shot with *Classic Chrome* (X-T2) and *Nostalgic Neg.* (X-T5)
- **When** their metadata is read
- **Then** the film simulation is reported with exactly those names

### CORE-001.4 – Thumbnail without RAW decoding
- **Given** any RAF from the reference set
- **When** a thumbnail is generated
- **Then** it is produced from the embedded JPEG, and the RAW decoder is not called

### CORE-001.5 – Full decode at 100% zoom
- **Given** each RAF from the reference set
- **When** it is decoded at full resolution
- **Then** the output has the sensor's full pixel size, and its hash matches the stored
  reference hash for the pinned decoder version and configuration

### CORE-001.6 – HEIF from the X-T5
- **Given** the reference file `fujifilm/x-t5/image.HIF`
- **When** its metadata is read and a thumbnail is generated
- **Then** the metadata match its `.expected.yaml`, and a thumbnail is produced

### CORE-001.7 – Damaged RAF
- **Given** an RAF truncated after the embedded JPEG
- **When** it is indexed
- **Then** the metadata and thumbnail are available, the asset is marked "damaged: RAW
  data incomplete", and no other asset is affected

### CORE-001.8 – Filter by film simulation
- **Given** a library with assets of three different film simulations
- **When** the user filters by *Classic Chrome*
- **Then** only the assets with *Classic Chrome* are shown

## Out of scope

- RAW + JPEG grouping: LIBRARY-001.
- Developing or editing RAW files: later editor phase.
- Applying film simulations to RAWs: not planned. The camera JPEG/HEIF carries the look.

## Open questions

- Does the X-T5 write `OffsetTimeOriginal`? If not, the time zone comes from the camera
  time correction (N15).

## History

| Date | Change |
|---|---|
| 2026-10-17 | Initial draft |
| 2026-10-18 | Reference set moved to `framehive-testdata` (ADR 0017) |
| 2026-11-09 | Own photos under the Framehive Test Data Licence instead of CC0 (owner decision) |
| 2026-11-09 | First sets published: X-T2 and X-T5 lossless RAF + JPG pairs; their reference decode hashes are tested under PREVIEWS-001.4 |
| 2026-11-10 | No further reference files by owner decision; unpublished cases have no fixture and no reference tests |
