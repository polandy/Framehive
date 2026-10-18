# CORE-004 – DNG support

- Status: draft
- Module: core (metadata and previews services)
- Origin: D1–D7, 2.7, 2.8, G2
- Depends on: LIBRARY-001 (grouping)

## Purpose

DNG is Adobe's open RAW format. The project owner's DNGs come from the Adobe DNG
Converter and from drones and action cameras. They must work fully, and their metadata
must stay visible in Lightroom when the user wants that.

## Behaviour

**Reference set.** It is published in the `framehive-testdata` repository and pinned
in `testdata.lock` as `dng/` (ADR 0017). It comes from raw.pixls.us and the project
owner's own files.

| Source | Files |
|---|---|
| Adobe DNG Converter | lossless; lossy; linear; without preview; with medium and full-size preview; with embedded original RAW; with embedded XMP |
| Drone | DJI DNG with GPS altitude and DJI XMP fields |
| Action camera | GoPro GPR |

**Reading DNG.** Our own TIFF/EXIF reader (shared with CR2, CORE-003) reads the IFD
tree: IFD0, the SubIFDs, EXIF and GPS IFDs, `DNGVersion`, `UniqueCameraModel`,
`NewSubFileType` and the embedded XMP (tag 700). The RAW decoder (LibRaw, ADR 0012) is
only used for full decoding.

**Metadata.** EXIF, GPS and the embedded XMP are read and reconciled per MWG.
`UniqueCameraModel` is shown if `Make`/`Model` are missing. Unknown XMP namespaces
(for example `drone-dji:`) are preserved unchanged.

**Writing metadata** (ADR 0015).

| Setting `dng_metadata` (YAML value) | Behaviour |
|---|---|
| `sidecar` (default) | Metadata goes to the sidecar. The DNG is never changed |
| `sidecar_and_embedded` | As `sidecar`; in addition the same XMP packet is written into the DNG, so that Lightroom sees it |

When reading, the sidecar and the embedded XMP are merged: values that exist only in
the embedded XMP are kept. If both contain a field and they differ, the one with the
newer `xmp:MetadataDate` wins, and the asset shows the conflict badge from LIBRARY-001.

Embedding works on a copy. The file is written as `<name>.part`, re-read, its raw
image digest (`NewRawImageDigest` or `RawImageDigest`, if present) is checked, and only
then is it renamed over the original. If any step fails, the original stays untouched
and the sidecar still has the change.

**Previews.** The thumbnail comes from the largest embedded preview (`NewSubFileType`
= 1). If the file has no preview, a RAW decode runs as a background job, and the tile
shows a placeholder until it is ready. The decode result is cached.

**Linear DNG** (already demosaiced, for example from DNG Converter's "linear" option
or a denoise step) is decoded without demosaicing.

**GoPro GPR.** Metadata and the embedded preview are read by our code. A full decode
depends on the RAW decoder being built with VC-5 support. If it is not, 100% zoom shows
the preview with a notice.

## Acceptance criteria

### CORE-004.1 – Metadata from every DNG variant
- **Given** every DNG in the reference set
- **When** its metadata is read
- **Then** all values match its `.expected.yaml`

### CORE-004.2 – Thumbnail from the largest preview
- **Given** a DNG with a medium and a full-size preview
- **When** a thumbnail is generated
- **Then** the full-size preview is used, and the RAW decoder is not called

### CORE-004.3 – DNG without preview
- **Given** a DNG without an embedded preview
- **When** it is shown in the grid
- **Then** a placeholder is shown, a decode job is queued, and the decoded thumbnail
  replaces the placeholder and is cached

### CORE-004.4 – Full decode
- **Given** the lossless, lossy and linear DNGs from the reference set
- **When** they are decoded at full resolution
- **Then** each output hash matches its stored reference hash for the pinned decoder
  version and configuration

### CORE-004.5 – Drone XMP is preserved
- **Given** a DJI DNG with `drone-dji:` fields in its embedded XMP
- **When** the user rates it with `dng_metadata: sidecar_and_embedded`
- **Then** the embedded XMP still contains every `drone-dji:` field unchanged

### CORE-004.6 – Default never changes the DNG
- **Given** a DNG and the default setting
- **When** the user rates and tags it
- **Then** the sidecar contains the rating and tags, and the DNG's bytes are unchanged

### CORE-004.7 – Embedding for Lightroom
- **Given** `dng_metadata: sidecar_and_embedded`
- **When** the user rates a DNG 4 stars
- **Then** the sidecar and the embedded XMP both contain `xmp:Rating` 4, exiftool reads
  the rating from the DNG, and the raw image digest is unchanged

### CORE-004.8 – Failed embedding leaves the original intact
- **Given** `dng_metadata: sidecar_and_embedded` and a fault injected after writing
  the `.part` file
- **When** the user rates a DNG
- **Then** the original DNG is byte-identical, no `.part` file remains, the sidecar has
  the rating, and the job reports the failure

### CORE-004.9 – Newer embedded XMP wins
- **Given** a sidecar with rating 2 and `xmp:MetadataDate` 10:00, and embedded XMP
  with rating 5 and `xmp:MetadataDate` 11:00 (edited in Lightroom)
- **When** the folder is indexed
- **Then** the asset has rating 5 and shows the conflict badge

### CORE-004.10 – Embedded-only fields are kept
- **Given** a DNG whose embedded XMP has a `dc:title`, and a sidecar without one
- **When** it is indexed
- **Then** the asset's title is the embedded title

### CORE-004.11 – GoPro GPR without VC-5 support
- **Given** a GPR file and a RAW decoder without VC-5 support
- **When** the user zooms to 100%
- **Then** the embedded preview is shown with the notice "Full resolution not available
  for this format"

## Out of scope

- Converting other RAW formats to DNG: not planned. The Adobe DNG Converter does that.
- Extracting the embedded original RAW: later, if needed.

## Open questions

- Does the pinned LibRaw version decode JPEG XL compressed DNG (DNG 1.7)? To be
  checked in phase 0. If not, the same fallback as for GPR applies.

## History

| Date | Change |
|---|---|
| 2026-10-18 | Initial draft |
| 2026-10-18 | Reference set in `framehive-testdata` (ADR 0017) |
