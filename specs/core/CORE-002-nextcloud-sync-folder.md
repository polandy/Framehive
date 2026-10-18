# CORE-002 – Nextcloud sync folder as library root

- Status: draft
- Module: core (volumes and catalog services); import uses it as a target
- Origin: NC1–NC6, S2, S3
- Depends on: –

## Purpose

The photos live in Nextcloud. On each machine the Nextcloud desktop client keeps a
sync folder, in which some files are local and some are online-only. Framehive must
work with that folder as its main library and import target, without ever forcing
the whole archive to download.

## Behaviour

**Detection.** A library root is recognised as a Nextcloud sync folder when the
client's sync database (`.sync_*.db`) is in its root or in one of its ancestors. The
root's `.framehive/library.yaml` records `kind: nextcloud_sync_folder` for display.
Recognition by UUID stays unchanged (S2).

**File availability.** Each file is in one of these states:

| State | Linux (Nextcloud client, virtual files) | macOS (File Provider) |
|---|---|---|
| `local` | Regular file | Regular file, not dataless |
| `online_only` | Placeholder `<name>.nextcloud` (suffix mode) | Dataless file: `SF_DATALESS` flag in `stat` |
| `absent` | Excluded by selective sync: the file does not exist locally | – |

The state is determined from names and `stat` only. Framehive never opens an
`online_only` file unless the user asks it to (the "Download" action, or "Open in"
an external program after a confirmation).

**Online-only assets** behave like assets on an offline drive (S3). They are
searchable, ratable and taggable. Changes go to the sidecar, which is small and is
synced by the client. Previews come from the local cache, or from Immich (IMMICH-002)
if that is configured; otherwise the tile shows a placeholder with the file type.

**Writing.** Sidecars and `.framehive/` files are written as `<name>.part` next to the
target and then renamed (N2). The Nextcloud client's temporary files (`.~*`,
`*.~*`), its databases (`.sync_*.db*`) and its logs (`.nextcloudsync.log`) are ignored
by scanning and watching.

**Conflict files.** A file whose name matches
`<base> (conflicted copy <YYYY-MM-DD> <hhmmss>).<ext>` is not indexed as its own
asset. It is attached to the asset with that base name as a conflict. For sidecars,
the user can merge field by field (S8) or choose one version. The losing file goes to
the trash.

## Acceptance criteria

### CORE-002.1 – Scanning does not download
- **Given** a Nextcloud root with 1,000 online-only files (simulated by the
  `FileAvailability` test fake)
- **When** the root is scanned and indexed
- **Then** no content of any online-only file is read, and every file has the state
  `online_only`

### CORE-002.2 – Linux placeholder is recognised
- **Given** a folder with `DSCF1234.RAF.nextcloud` and `DSCF1234.JPG`
- **When** it is scanned
- **Then** there is one asset with the members `DSCF1234.RAF` (`online_only`) and
  `DSCF1234.JPG` (`local`)

### CORE-002.3 – macOS dataless file is recognised
- **Given** on macOS a file whose `stat` flags contain `SF_DATALESS`
- **When** it is scanned
- **Then** its state is `online_only`, and its content is not read
- (Runs in the macOS CI only.)

### CORE-002.4 – Rating an online-only asset
- **Given** an asset whose RAF is `online_only`
- **When** the user rates it 3 stars
- **Then** the sidecar is written with `xmp:Rating` 3, and the RAF stays `online_only`

### CORE-002.5 – Thumbnail without download
- **Given** an online-only asset without a cached preview, and Immich not configured
- **When** the grid shows it
- **Then** the tile shows a placeholder with the file type and a cloud icon, and no
  download is triggered

### CORE-002.6 – Explicit download
- **Given** an online-only asset
- **When** the user chooses "Download"
- **Then** the file is read, which makes the client fetch it, and the state becomes
  `local` once the read succeeds

### CORE-002.7 – Conflict copy of a sidecar
- **Given** `DSCF1234.RAF.xmp` (rating 2, tag `A`) and
  `DSCF1234.RAF (conflicted copy 2026-10-17 101500).xmp` (rating 4, tag `B`)
- **When** the folder is indexed
- **Then** there is one asset with a conflict, and "Merge" produces rating 4 (the newer
  file) and tags `A` + `B`, with the conflict copy moved to the trash

### CORE-002.8 – Client files are ignored
- **Given** a root containing `.sync_abc123.db`, `.sync_abc123.db-wal` and
  `.nextcloudsync.log`
- **When** it is scanned
- **Then** none of them appears in the index

### CORE-002.9 – Import into the sync folder
- **Given** a profile whose destination is inside a Nextcloud root
- **When** an import finishes
- **Then** every file is written as `.part` and renamed, and no `.part` file remains

## Out of scope

- Talking to the Nextcloud server (WebDAV): not planned. The desktop client does the
  syncing.
- Windows Cloud Files API: only if Windows is supported.

## Open questions

- Which Nextcloud client versions and virtual-file modes are supported on Linux?
  Suffix mode is experimental there. This needs verifying with the owner's setup.
- How are online-only files detected in the macOS File Provider domain without the
  provider hydrating them during `readdir`? To be verified in a phase 0 prototype.

## History

| Date | Change |
|---|---|
| 2026-10-17 | Initial draft |
