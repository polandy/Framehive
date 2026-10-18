# 4. Improvement Proposals [NEW]

Part of the [requirements](README.md).

## 4.1 Data safety

| ID | Requirement | Priority |
|---|---|---|
| N1 | **Verified copy**: compute a checksum (BLAKE3/xxh3) while reading, and check it again after writing at the destination and at each backup | M |
| N2 | **Atomic writes**: write to a `.part` file, fsync, then rename. Never a partial file at the destination | M |
| N3 | **Content-based recognition**: import history in SQLite (hash + metadata). Replaces the timezone workaround and recognizes files even after renaming | M |
| N4 | **Resuming**: an interrupted import can be continued; already-verified files are skipped | S |
| N5 | **"Card can be formatted"**: the device gets a green status as soon as every file is verified in N locations (N configurable, e.g. 2 for destination + 1 backup) | M |
| N6 | **Import log**: per run, traceable record of which file went where, with checksums (exportable as JSON/CSV) | S |
| N7 | Optional: treat the card as read-only after import, never write to the source | M |

## 4.2 Usability

| ID | Requirement | Priority |
|---|---|---|
| N8 | **Profiles/presets**: destination, naming, folders, backup, metadata as a named profile ("Vacation", "Client job", "Phone"). A profile can be automatically bound to a device | M |
| N9 | **Template language with live preview**: e.g. `{date:%Y}/{date:%Y-%m-%d}_{job}/{date:%Y%m%d}-{seq:4}.{ext}`. Graphical block editor *and* text mode; preview shows 5 real files from the current selection including conflict detection | M |
| N10 | **Large preview / loupe** with keyboard control (space bar, arrow keys) and **culling** (pick/reject, star ratings). Reject = not imported; stars are written to XMP | M |
| N11 | **Grouping**: RAW+JPEG as one asset (B23–B28, M); burst shots and exposure sequences as stacks in the grid (B5, S) | M/S |
| N12 | Progress with throughput (MB/s), ETA, per device and per destination | M |
| N13 | Desktop notification on completion/error | S |
| N14 | Keyboard-first: every action via shortcut, command palette | S |

## 4.3 Metadata

| ID | Requirement | Priority |
|---|---|---|
| N15 | **Camera time correction**: offset per camera (wrong clock, wrong timezone), aligning multiple cameras using a reference photo | S |
| N16 | **Write XMP sidecars**: copyright, creator, keywords, job code, rating, from the profile | S |
| N17 | **Geotagging** from a GPX track or from a smartphone photo with GPS | K |

## 4.4 Integration & Automation

| ID | Requirement | Priority |
|---|---|---|
| N18 | **Headless CLI** with the same core: `framehive import --profile vacation /run/media/…` | M |
| N19 | **Background service** (optional): import on plug-in without an open window, tray/notification | K |
| N20 | **Post-import hooks**: open the folder in darktable/digiKam, run a custom script | S |
| N21 | Backup destinations via rclone (S3, NAS, cloud) | K |

## 4.5 Platform & Technology

| ID | Requirement | Priority |
|---|---|---|
| N22 | **Linux and macOS** as target platforms, Linux first. Windows undecided; device access abstracted behind a trait | M |
| N23 | **Few dependencies** (ADR 0009): metadata handling in pure Rust where possible, custom code for the needed EXIF fields, thumbnails from the RAWs' embedded JPEG previews (fast, no demosaicing). ffmpeg only for videos, optional | M |
| N24 | **Packaging**: Flatpak, AUR, static binary | S |
| N25 | Configuration as YAML, readable and version-controllable (ADR 0011) | M |
| N26 | Fault tolerance: a single broken file never aborts the whole import; parsers run in isolation | M |

## 4.6 Background jobs

| ID | Requirement | Priority |
|---|---|---|
| J1 | An import is a **job in a queue** in the core; the UI never blocks | M |
| J2 | While an import is running, you can view the next source, cull it, and queue it as another job | M |
| J3 | Jobs with different sources and destinations run in parallel; sequence counters and destination names are reserved at queue time so parallel jobs don't create conflicts | M |
| J4 | Activity indicator in the window header listing all jobs (import, catching up on backup, thumbnails, indexing, faces, hashing): progress, pause, cancel, reorder | M |
| J5 | Imported images appear in the library immediately once they are verified | M |
| J6 | The queue survives an app restart and resumes (N4) | S |
| J7 | The "card can be formatted" notification appears even when the import module isn't open | M |
| J8 | Low-priority jobs (AI, hashing, previews) automatically pause during an import and on battery power | S |
