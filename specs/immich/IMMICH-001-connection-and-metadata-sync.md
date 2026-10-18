# IMMICH-001 – Connection and metadata sync

- Status: draft
- Module: immich
- Origin: IM1–IM5, IM8–IM10, B19
- Depends on: CORE-002 (Nextcloud sync folder), LIBRARY-001 (assets)

## Purpose

Immich has already recognised and named faces, and it holds albums, tags, favorites
and places. Framehive takes this knowledge over into standard XMP, so it is available
offline and in other programs. Immich itself is never changed.

## Behaviour

**Configuration** (`immich` section of the application settings):

```yaml
immich:
  server_url: https://immich.home.example
  api_key_file: ~/.config/framehive/immich.key   # never the key itself in YAML
  path_mappings:
    - immich_path: /data/nextcloud/andy/files/
      local_path: ~/Nextcloud/
  sync:
    faces: true
    albums: true
    tags: true
    favorites_as_tag: Immich/Favorite
    ratings: true
    descriptions: true
    places: true
    run_automatically_every: 24 h
```

**Connecting.** Framehive checks the server version against the supported range (ADR
0013) and verifies the key with a read-only request. A version outside the range
disables the module and shows a message.

**Matching.** For each Immich asset:

1. Its original path is translated with `path_mappings`. If a local asset member
   exists at that path, they match.
2. Otherwise, if a local file with the same original file name and capture time is
   `local`, its SHA-1 is compared with Immich's checksum.
3. Otherwise the asset is listed as unmatched in the sync report.

**Sync** runs as a background job, either manually or on the configured interval. It
writes into the sidecar of every member of the matched asset:

| Immich | XMP |
|---|---|
| Face with a named person | `mwg-rs:Region`, type `Face`, `mwg-rs:Name` = person name; area converted from Immich's box (pixels in `imageWidth`×`imageHeight`) to MWG normalised coordinates, respecting EXIF orientation |
| Face without a name | Not written |
| Tags (`a/b/c`) | `lr:hierarchicalSubject` `a|b|c` (pipe-separated) and `dc:subject` `c` |
| Favorite | Tag from `favorites_as_tag` |
| Rating 1–5 | `xmp:Rating` |
| Description | `dc:description` (`x-default`) |
| City / state / country | `photoshop:City` / `photoshop:State` / `photoshop:Country` |
| Album | Framehive album `.framehive/albums/immich-<album-id>.yaml`, `source: immich`, read-only in Framehive |

**Ownership.** For every value it writes, the sync stores the field and a hash of the
value in Framehive's XMP namespace (`docs/xmp-namespace.md`). On the next sync:

| Current value in XMP | Immich value | Result |
|---|---|---|
| Empty | Present | Written, recorded as sync-owned |
| Sync-owned, unchanged | Changed | Updated |
| Sync-owned, unchanged | Removed | Removed |
| Changed by the user (hash differs) or never sync-owned | Any | Kept; ownership ends; the report lists it |

Tags and regions are compared per entry, not per field. A user's own tags stay when
Immich tags are added or removed.

**Read-only guarantee.** The module can only send the requests listed in ADR 0013.
Anything else is rejected by the adapter before it reaches the network.

## Acceptance criteria

### IMMICH-001.1 – Version outside the supported range
- **Given** a fake Immich reporting a version above the supported range
- **When** the user connects
- **Then** the module is disabled, and the message names the found and supported
  versions

### IMMICH-001.2 – Matching by path mapping
- **Given** an Immich asset with original path
  `/data/nextcloud/andy/files/Photos/2026/DSCF1234.RAF`, and a local file
  `~/Nextcloud/Photos/2026/DSCF1234.RAF`
- **When** the sync runs
- **Then** the Immich asset is matched to that local asset

### IMMICH-001.3 – Checksum fallback never downloads
- **Given** an Immich asset with no path match, whose candidate local file is
  `online_only`
- **When** the sync runs
- **Then** the file is not read, and the asset is listed as unmatched

### IMMICH-001.4 – Named face becomes an MWG region
- **Given** an Immich face for person "Anna" with box (100, 50)–(300, 350) in a
  1000×1000 image without rotation
- **When** the sync runs
- **Then** the sidecar contains a `Face` region named "Anna" with center (0.2, 0.2),
  width 0.2 and height 0.3

### IMMICH-001.5 – Rotated image
- **Given** the same face on an image with EXIF orientation 6 (rotated 90° clockwise)
- **When** the sync runs
- **Then** the region coordinates are transformed as MWG 2.0 requires, checked against
  the reference values that digiKam and Lightroom read for the same file

### IMMICH-001.6 – Unnamed faces are skipped
- **Given** an Immich face without a person name
- **When** the sync runs
- **Then** no region is written for it

### IMMICH-001.7 – Hierarchical tag
- **Given** an Immich tag `Places/Italy/Dolomites` on an asset
- **When** the sync runs
- **Then** `lr:hierarchicalSubject` contains `Places|Italy|Dolomites`, and `dc:subject`
  contains `Dolomites`

### IMMICH-001.8 – Favorite as tag
- **Given** an asset marked as favorite in Immich
- **When** the sync runs with the default settings
- **Then** the asset has the tag `Immich/Favorite`, and its rating is unchanged

### IMMICH-001.9 – User edits are kept
- **Given** a description written by an earlier sync, which the user then edited in
  Framehive
- **When** Immich's description changes and the sync runs again
- **Then** the user's description is kept, and the report lists the asset

### IMMICH-001.10 – Removed in Immich, removed locally
- **Given** a tag written by an earlier sync and unchanged since
- **When** the tag is removed in Immich and the sync runs again
- **Then** the tag is removed from the sidecar, and the user's own tags remain

### IMMICH-001.11 – Sync is idempotent
- **Given** a completed sync
- **When** the sync runs again without changes in Immich
- **Then** no sidecar is written (modification times are unchanged)

### IMMICH-001.12 – Album mirror is read-only
- **Given** an Immich album "Summer 2026" with 30 matched assets
- **When** the sync runs
- **Then** Framehive shows the album with 30 assets and an Immich badge, and the
  commands that change albums are disabled for it

### IMMICH-001.13 – No write requests
- **Given** the adapter
- **When** any code path asks it to send a request other than the allowed read
  requests
- **Then** the request is rejected before it is sent (conformance test over all
  `ImmichApi` methods)

### IMMICH-001.14 – Offline Immich
- **Given** a configured Immich that cannot be reached
- **When** the automatic sync is due
- **Then** the job ends with "Immich unreachable", is retried at the next interval,
  and nothing else in the library is affected

## Out of scope

- Content search and previews from Immich: IMMICH-002.
- Local face recognition: later provider of the people module.

## Open questions

- Does the pinned Immich version re-import face regions from Framehive's sidecars
  (risk in ADR 0013)?
- Where is the API key stored on macOS: in a file, or in the Keychain via an adapter?

## History

| Date | Change |
|---|---|
| 2026-10-17 | Initial draft |
