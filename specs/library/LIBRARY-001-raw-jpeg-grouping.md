# LIBRARY-001 – RAW + JPEG grouping

- Status: draft
- Module: library (grouping rule in the catalog service, used by import and library)
- Origin: B23, B24, B25, B26, B27, B28, G8, N11, R5, F5, F6
- Depends on: IMPORT-002 (sequence numbers)

## Purpose

Many photographers shoot RAW + JPEG: the JPEG carries the camera's look (for example a
Fujifilm film simulation), and the RAW keeps all the data for editing. In the library
they are one picture. Rating or deleting it must not mean doing the same thing twice,
and the two files must never drift apart.

## Behaviour

**Grouping rule.** Files form one asset when all of these hold:

- They are in the same folder.
- Their base names are equal, compared case-sensitively. The base name is the file
  name up to the last `.`.
- At most one member has a RAW extension (2.7), and at most one has a rendered
  extension (`JPG`, `JPEG`, `HEIF`, `HEIC`, `HIF`, `TIF`, `TIFF`). Extensions are
  compared case-insensitively.

Sidecars (`.xmp`) belong to the asset of the file they describe. A `.THM` file
belongs to the video with the same base name, with role `Sidecar` (CORE-003). If a folder contains
`A.JPG`, `A.jpg` and `A.RAF`, there is no unique rendered member. Then every file
becomes its own asset, and the library shows a warning.

The grouping is computed from file names alone. It is not stored, so rebuilding the
index gives the same assets.

**Display modes** (per view, remembered):

| Mode (YAML value) | Grid shows |
|---|---|
| `grouped` (default) | One tile per asset, badge with member types (`RAF+JPG`) |
| `raw_only` | Assets with a RAW member; preview from the RAW |
| `rendered_only` | Assets with a rendered member |
| `all_files` | One tile per file, like a file manager |

**Metadata.** Rating, pick state, color label, tags, title, description and regions
belong to the asset. A change is written to the sidecar of every member. When sidecar
naming is `original_without_extension`, the members share the single sidecar
`DSCF1234.xmp`, which is the Adobe convention.

When the index is built and the members' sidecars disagree, the most recently modified
sidecar wins. Ties go to the RAW member's sidecar. The other sidecars are rewritten on
the next write. The conflict is logged, and the asset shows a small conflict badge
until the user dismisses it.

**File operations.** Move, rename and delete (to trash) act on all members and their
sidecars together. "Delete rendered member" and "Delete RAW member" are separate,
explicit commands. They need a confirmation that names the file type.

**Import.** The import plan uses the same grouping rule on the source. Members of an
asset get the same sequence number, target folder and base name. With dual-slot "RAW /
JPEG separation" (F6), the members sit on different cards. They are matched by camera
serial number, original base name and capture time to the second. This also works
across separate import jobs: a later job finds the counterpart in the import history
and places its file next to it, with the same base name.

## Acceptance criteria

### LIBRARY-001.1 – RAW and JPEG form one asset
- **Given** a folder with `DSCF1234.RAF` and `DSCF1234.JPG`
- **When** the library indexes the folder
- **Then** there is exactly one asset with two members, roles `Raw` and `Rendered`

### LIBRARY-001.2 – HEIF pairs like JPEG
- **Given** a folder with `DSCF1234.RAF` and `DSCF1234.HIF`
- **When** the library indexes the folder
- **Then** there is exactly one asset with roles `Raw` and `Rendered`

### LIBRARY-001.3 – Different folders do not group
- **Given** `a/DSCF1234.RAF` and `b/DSCF1234.JPG`
- **When** the library indexes both folders
- **Then** there are two assets

### LIBRARY-001.4 – Ambiguous members do not group
- **Given** a folder with `A.RAF`, `A.JPG` and `A.jpg` on a case-sensitive file system
- **When** the library indexes the folder
- **Then** there are three assets, and a warning names the folder and the base name `A`

### LIBRARY-001.5 – Grouping survives an index rebuild
- **Given** an indexed library with grouped assets
- **When** the index is deleted and rebuilt
- **Then** the assets and their members are identical (T15)

### LIBRARY-001.6 – Rating is written to every member
- **Given** an asset `DSCF1234.RAF` + `DSCF1234.JPG` with sidecar naming
  `original_with_extension`
- **When** the user rates it 4 stars
- **Then** `DSCF1234.RAF.xmp` and `DSCF1234.JPG.xmp` both contain `xmp:Rating` 4

### LIBRARY-001.7 – Shared sidecar without extension
- **Given** the same asset with sidecar naming `original_without_extension`
- **When** the user rates it 4 stars
- **Then** exactly one sidecar `DSCF1234.xmp` exists, and it contains `xmp:Rating` 4

### LIBRARY-001.8 – Newest sidecar wins on disagreement
- **Given** `DSCF1234.RAF.xmp` with rating 2, modified at 10:00, and `DSCF1234.JPG.xmp`
  with rating 5, modified at 11:00
- **When** the library indexes the folder
- **Then** the asset has rating 5, the conflict is logged, and the asset shows a
  conflict badge

### LIBRARY-001.9 – Delete acts on the whole asset
- **Given** an asset with `.RAF`, `.JPG` and their sidecars
- **When** the user deletes the asset
- **Then** all four files are in the trash

### LIBRARY-001.10 – Delete only the rendered member
- **Given** an asset with `.RAF` and `.JPG`
- **When** the user runs "Delete rendered member" and confirms
- **Then** the `.JPG` and its own sidecar are in the trash, and the asset remains with
  the `Raw` member and its metadata

### LIBRARY-001.11 – Display mode raw only
- **Given** assets `A` (RAF+JPG), `B` (JPG only) and `C` (RAF only)
- **When** the view is set to `raw_only`
- **Then** the grid shows `A` and `C`

### LIBRARY-001.12 – Import keeps members together
- **Given** a card with `DSCF1234.RAF` and `DSCF1234.JPG`, and the template
  `{date:%Y%m%d}_{seq_session:4}`
- **When** they are imported
- **Then** both get the same new base name, and they land in the same folder

### LIBRARY-001.13 – Dual-slot separation is paired on import
- **Given** card 1 with `DSCF1234.RAF` and card 2 with `DSCF1234.JPG`, both from the
  same camera serial number with the same capture time
- **When** both cards are imported in one job
- **Then** the files form one asset at the target, with the same sequence number

### LIBRARY-001.14 – Dual-slot separation is paired across jobs
- **Given** card 1 with `DSCF1234.RAF` was imported yesterday as `20261016_0042.RAF`
- **When** card 2 with `DSCF1234.JPG` (same camera serial number, same capture time) is
  imported today in a new job
- **Then** the JPEG is stored as `20261016_0042.JPG` in the same folder, and both form
  one asset

## Out of scope

- Bursts, exposure brackets and versions as stacks: separate spec (B5).
- Which member is the preview and which goes to external programs (B28): separate spec.
- GoPro LRV/GPR and iOS Live Photo pairing (G2, G9).

## Open questions

–

## History

| Date | Change |
|---|---|
| 2026-10-17 | Initial draft |
| 2026-10-17 | Decided: newest sidecar wins with a badge; pairing across jobs (.14) |
| 2026-10-18 | `.THM` files join their video's asset (CORE-003) |
