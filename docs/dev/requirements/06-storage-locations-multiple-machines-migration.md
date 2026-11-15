# 6. Storage locations, multiple machines, migration

Part of the [requirements](README.md).

## 6.1 Storage locations

| ID | Requirement | Priority |
|---|---|---|
| S1 | Multiple library roots: local disk, NAS (SMB/NFS), external drives | M |
| S2 | Roots are recognized by an ID (marker file `.framehive/library.yaml` with a UUID), not by mount path. `/mnt/nas/photo` on Linux and `/Volumes/photo` on macOS are the same root | M |
| S3 | **Offline drives**: images remain searchable, ratable, and taggable via their preview; changes are queued and written to XMP the next time the drive is connected | M |
| S4 | Local preview cache per machine: thumbnails and larger previews (~2560 px) for the loupe, size configurable | M |
| S5 | NAS is slow: reads go through the cache, writes are asynchronous via the job queue | M |
| S6 | Detect external changes: file watcher on local disks, periodic reconciliation (mtime/size of the XMP) on NAS | S |

## 6.2 Multiple machines (Linux + Mac)

| ID | Requirement | Priority |
|---|---|---|
| S7 | Each machine has its own local index. The SQLite database is **never** on the NAS (SQLite over SMB/NFS is not safe). Only what lives in files is shared: XMP and `.framehive/` | M |
| S8 | Simultaneous changes on two machines: field-level merge (tags are unioned; for rating, the newer change wins); surface genuine conflicts | S |
| S9 | Faces and place names travel via XMP; embeddings and hashes are a per-machine cache, optionally shared under `.framehive/cache/` | K |

## 6.3 Migration

| ID | Requirement | Priority |
|---|---|---|
| M1 | **Migrate from digiKam**: read `digikam4.db` (SQLite; MySQL K): tag hierarchy, ratings, color and pick labels, albums, face regions and people, title and description. Write the result as XMP | M |
| M2 | Read existing folders with XMP: sidecars from digiKam, darktable, and Lightroom (`image.xmp` and `image.ARW.xmp`), embedded XMP/IPTC | M |
| M3 | Migration always starts as a dry run with a report (what is carried over, what is lost, which conflicts); nothing changes before the user confirms | M |

## 6.4 Nextcloud sync folders [NEW]

The project owner's photos live in Nextcloud. Framehive machines see them through the
Nextcloud desktop client's sync folder, and that folder is the main library root and
the import target. Binding behaviour: `specs/core/CORE-002-nextcloud-sync-folder.md`.

| ID | Requirement | Priority |
|---|---|---|
| NC1 | A Nextcloud sync folder can be a library root and the import target. New imports are uploaded by the Nextcloud client; Framehive does not talk to Nextcloud directly | M |
| NC2 | **Online-only files** (Linux: virtual-file placeholders; macOS: File Provider "dataless" files) are detected without reading their content. Scanning, indexing and thumbnail generation never trigger a download | M |
| NC3 | Online-only files appear in the library like offline drives (S3): searchable, ratable, taggable; previews from the cache or from Immich (IM6); "Download" is an explicit action (e.g. before opening in darktable) | M |
| NC4 | Nextcloud conflict files (`name (conflicted copy <date>).ext`) are detected, shown as a conflict, and offered for merging (sidecars: field-level merge, S8) | S |
| NC5 | Files written by Framehive (sidecars, `.framehive/`) are written atomically, so the Nextcloud client never uploads half-written files; the client's own temporary and database files are ignored | M |
| NC6 | The same Nextcloud folder synced on several machines is the same library root (S2) | M |

## 6.5 Immich (read-only) [NEW]

The long-term archive is Immich. It indexes the same Nextcloud data as an external
library, mounted read-only. Framehive uses what Immich already knows, especially faces,
and never writes to Immich (ADR 0013). Binding behaviour: `specs/immich/`.

| ID | Requirement | Priority |
|---|---|---|
| IM1 | Connect to an Immich server with a read-only API key; check the server version against the supported range | S |
| IM2 | Match Immich assets to local files: path mapping (container path → local path, in YAML), fallback SHA-1 checksum for locally synced files; report unmatched assets | S |
| IM3 | **Faces and people** from Immich written as MWG face regions with names into the sidecars; person filter in the library | S |
| IM4 | Albums, tags, favorites (as tag `Immich/Favorite`), ratings, descriptions written into XMP and Framehive albums | S |
| IM5 | Places from Immich's reverse geocoding written as IPTC location fields (`photoshop:City`, `photoshop:State`, `photoshop:Country`) | S |
| IM6 | Previews for online-only files from Immich thumbnails | S |
| IM7 | **Content search** ("dog on the beach") via Immich smart search, results shown as local assets | S |
| IM8 | Sync is one-way, repeatable and never overwrites values the user changed in Framehive | S |
| IM9 | Without a connection to Immich everything else keeps working; Immich features show "unavailable" | S |
| IM10 | Never any write access to Immich (no upload, no edit, no delete) | M |
