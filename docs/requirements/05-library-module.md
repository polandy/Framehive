# 5. Library module

Part of the [requirements](README.md).

## 5.1 Data model

- Metadata (rating, pick/reject, color label, tags, title, description, copyright,
  face regions, GPS) is written to **XMP sidecars**. Naming is configurable:
  `image.ARW.xmp` (darktable, digiKam) or `image.xmp` (Lightroom). Writing directly
  into JPEG/HEIF is optional, and into DNG optional per ADR 0015; never for other RAW
  formats.
- The SQLite database is an **index** per machine: fast search, filters, thumbnails,
  embeddings. It can be fully rebuilt from the files.
- Anything that doesn't fit in XMP (albums, saved searches, library settings) is
  stored as a file in the library under `<root>/.framehive/` and travels with it.
- Changes made by other programs or machines are detected and picked up (6.1).

## 5.2 Requirements

| ID | Requirement | Priority |
|---|---|---|
| **Browsing** | | |
| B1 | Folder view of the library roots | M |
| B2 | Date view: year / month / day, calendar with frequency | M |
| B3 | Virtualized grid with continuously adjustable size, info overlay (rating, label, file type) | M |
| B4 | Loupe and full screen; side-by-side comparison of 2–4 images, synchronized zoom | M |
| B5 | Stacks: bursts, exposure sequences, versions (RAW+JPEG is not a stack but one asset, B23) | S |
| **Rating & Tagging** | | |
| B6 | Pick/reject, 0–5 stars, color labels, all via keyboard (P, X, 1–5, 6–9) with auto-advance | M |
| B7 | Hierarchical tags (`Places/Italy/Dolomites`), autocomplete, assignment via keyboard and drag & drop, tag tree for renaming and merging | M |
| B8 | Change metadata for many images at once | M |
| B9 | Title, description, creator, copyright (IPTC Core) | M |
| B10 | Retroactive time correction for a selection (camera offset, timezone) | S |
| **Organizing** | | |
| B11 | Albums (manually assembled) | M |
| B12 | Smart collections (saved searches) | S |
| B13 | Move and rename files within the library (using the template engine from import); sidecars move with them; deleting goes to the trash; "clean up rejects" | M |
| **Searching & Filtering** | | |
| B14 | Filter bar: rating, pick/reject, label, tag, camera, lens, date, ISO, aperture, focal length, file type, location, person | M |
| B15 | Full-text search across file name, title, description, tags | M |
| **Hand-off** | | |
| B16 | Open in an external program (darktable, RawTherapee, GIMP, configurable) | M |
| B17 | Export as JPEG/AVIF (size, quality, metadata filter, watermark); RAWs via embedded preview or `darktable-cli` | S |
| **Advanced features (soon after v1)** | | |
| B18 | **Map**: images with GPS shown on OpenStreetMap, geotagging via GPX track or by dragging onto the map, place names as tags via offline reverse geocoding | S |
| B19 | **Faces**: from Immich first (IM3), local detection and recognition later as a second provider; naming people, confirming suggestions; stored as MWG regions in XMP (compatible with digiKam and Lightroom) | S |
| B20 | **Duplicates & similarity**: exact duplicates via BLAKE3, similar images via perceptual hash, burst detection with a "sharpest image in the series" suggestion for culling | S |
| B21 | **Content search**: search by description ("dog on the beach") and "find similar images"; via Immich first (IM7), local CLIP embeddings later | S |
| B22 | All AI features run **locally**, are optional, models are downloaded on demand, GPU acceleration when available (CUDA, Metal/CoreML); no cloud | M |

## 5.3 RAW + JPEG grouping [NEW]

A key feature. A RAW file and the JPEG/HEIF the camera wrote for the same shot form one
**asset** (glossary). Binding behaviour: `specs/library/LIBRARY-001-raw-jpeg-grouping.md`.

| ID | Requirement | Priority |
|---|---|---|
| B23 | Files with the same base name in the same folder, differing only in extension (e.g. `DSCF1234.RAF` + `DSCF1234.JPG` / `.HIF`), are grouped into one asset automatically. The grouping is derived from the files, not stored, so the index stays rebuildable | M |
| B24 | Import keeps groups together: same sequence number (R5), same target folder and base name, even if RAW and JPEG come from different cards (dual slot, F6) | M |
| B25 | The grid shows one tile per asset with a badge (`RAF+JPG`); the user can choose per library or per view: show grouped, show RAW only, show JPEG only, show all files separately | M |
| B26 | Rating, pick state, color label, tags and other metadata always apply to the whole asset and are written to every member's sidecar consistently | M |
| B27 | Delete/move/rename act on the whole asset by default; "delete JPEG only" / "delete RAW only" are explicit commands (e.g. keep only the RAW after culling) | M |
| B28 | Choose which member is shown as the preview (default: JPEG, because it carries the in-camera look such as the film simulation) and which one external programs and export use (default: RAW) | S |
