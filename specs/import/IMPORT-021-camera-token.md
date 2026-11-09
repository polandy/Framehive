# IMPORT-021 – Camera token

- Status: implemented
- Module: import
- Origin: R2
- Depends on: IMPORT-001

## Purpose

Photographers who shoot with several cameras want to sort files into a folder or a name
per camera. The `{camera}` token of a name template shows the camera model from the EXIF.

## Behaviour

**Token.** `{camera}` takes no option. It is valid in the name template and in the video
name template (IMPORT-018).

**Source.** The value is the EXIF `Model` tag (0x0110, IFD0) of the file, read from the
same containers as the capture time (CORE-005): JPEG and MPO, TIFF-based RAW, RAF. As for
the capture time, all files of an asset share one value: the one of the RAW file if it
has a model, otherwise of the first other file that has one (IMPORT-001, "Assets").

**Value.** The text is cut at its first NUL and trimmed. Then, in every mode:

- `/ \ < > : " | ? *` and control characters other than blanks become `_`, so the value never splits a
  segment and is valid on every file system,
- runs of blanks (space, tab, line breaks) become one space, and leading and trailing blanks and dots are removed,
- at most 64 characters are kept, then trailing blanks and dots are removed again.

Compatibility mode (IMPORT-001) then applies to the segment as usual.

**Fallback.** A file without a `Model` tag, with an empty or damaged one, in a format
that is not read, or whose value is empty after the steps above, gives `Unknown camera`.
A file that cannot be read at all fails as for the capture time.

## Acceptance criteria

### IMPORT-021.1 – Camera model in a name
- **Given** `DSC_0042.RAF` from a camera whose model is `X-T5`, and the template
  `{camera}/{name}.{ext}`
- **When** the import plan is made
- **Then** the path is `X-T5/DSC_0042.RAF`

### IMPORT-021.2 – Missing model
- **Given** an asset whose files have no camera model, and the template
  `{camera}/{name}.{ext}`
- **When** the import plan is made
- **Then** the path is `Unknown camera/DSC_0042.RAF`

### IMPORT-021.3 – Characters that are invalid in file names
- **Given** the model `A/B:C*"D"?` and the template `{camera}/{name}.{ext}`, with
  compatibility mode on and off
- **When** the import plan is made
- **Then** the folder is `A_B_C__D__` in both modes

### IMPORT-021.4 – Blanks and dots
- **Given** the model `  Canon   EOS  7D. . ` and the template `{camera}/{name}.{ext}`
- **When** the import plan is made
- **Then** the folder is `Canon EOS 7D`; a model of only blanks and dots gives the
  fallback

### IMPORT-021.5 – Very long model
- **Given** a model of 200 characters
- **When** the import plan is made
- **Then** the value has 64 characters, cut on a character boundary

### IMPORT-021.6 – One camera for all files of an asset
- **Given** `DSC_0042.RAF` with the model `X-T5` and `DSC_0042.JPG` with the model
  `Other`, or only the JPEG with a model
- **When** the import plan is made
- **Then** both files use `X-T5` in the first case, and the JPEG's model in the second

### IMPORT-021.7 – The token takes no option
- **Given** the template `{camera:upper}/{name}.{ext}`
- **When** the user saves it
- **Then** it is rejected, and the error names `{camera:upper}`

### IMPORT-021.8 – Reading the model from the EXIF
- **Given** a JPEG, a TIFF-based RAW in either byte order and a RAF whose IFD0 has a
  `Model`, and files without one, with a damaged EXIF or in a format that is not read
- **When** the camera model is read
- **Then** the first three give the model; the others give none and no error

### IMPORT-021.9 – Files of a real camera
- **Given** the Lightroom DNG and the camera JPEG of the reference set
  `dng/canon-eos-350d/` (CORE-004)
- **When** their camera model is read
- **Then** both give `Canon EOS 350D DIGITAL`

### IMPORT-021.10 – Files of the reference cameras
- **Given** the RAW or DNG file and the camera JPEG of the reference sets
  `fujifilm/x-t2/`, `fujifilm/x-t5/` (CORE-001) and `canon/eos-7d/` (CORE-003)
- **When** their camera model is read
- **Then** they give `X-T2`, `X-T5` and `Canon EOS 7D`

## Out of scope

- Other metadata tokens (lens, make): later specs.
- Camera time correction by model (N15).

## Open questions

–

## History

| Date | Change |
|---|---|
| 2026-11-09 | Initial draft, accepted by the owner |
| 2026-11-09 | IMPORT-021.9 added and implemented: the owner's Canon EOS 350D DNG and JPEG |
| 2026-11-09 | IMPORT-021.10 added and implemented: the owner's X-T2, X-T5 and EOS 7D files |
