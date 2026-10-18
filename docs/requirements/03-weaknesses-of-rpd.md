# 3. Weaknesses of RPD

Part of the [requirements](README.md).

Derived from documentation, bug-tracker patterns, and architecture:

1. **Python + Qt5 + many native dependencies** (exiv2, ExifTool, gphoto2, pyheif, …):
   hard to package, breaks on distro updates, crashes on certain DNGs.
2. **Recognition by name + size + mtime** instead of content: needs the timezone
   workaround, fails after files are renamed on the card.
3. **No verification of copies**: no checksum comparison between source, destination,
   and backup.
4. **No resuming** after an interruption (cable pulled, battery dead).
5. **Linux only** (Windows only in a limited way via WSL, no cameras).
6. **Usability**: the rename editor is a modal dialog with chained dropdowns; preview
   only for a single example file; no profiles.
7. **No real pre-culling**: no large preview, no culling.
8. **No metadata enrichment** (copyright, keywords, GPS).

## 3.1 From the GitHub Issue Tracker

As of 2026-10-17: [damonlynch/rapid-photo-downloader](https://github.com/damonlynch/rapid-photo-downloader),
GPL-3.0, 37 open issues, last push 2026-08. The most-discussed open issues:

**Bugs the architecture must prevent**
- One CPU core pinned at 100%, memory leak during thumbnail generation up to a system
  freeze → Rust core with bounded worker pools and backpressure.
- Import never finishes, thumbnail generation hangs at 4% (Sony RX100), video import
  broken → per-file timeouts; a single file must never block the run (N26).
- Timeline splits one day's images across two days, video date wrong (parsing of
  `encoded_date … UTC`) → proper timezone handling for videos (QuickTime time is UTC).
- "Invalid cross-device link" → copy instead of `rename` across filesystem boundaries.
- Unknown size on SFTP mounts blocks the import → space check must only warn when the
  size is unknown.
- No Flatpak, PyQt6 port pending, problems under WSL/Wayland.

**Requested features (adopted as requirements)**

| ID | Requirement | Priority |
|---|---|---|
| G1 | **Move mode**: delete source files after a verified import (only after N1/N5, with confirmation) | S |
| G2 | GoPro: recognize and pair `.LRV` (proxy) and `.GPR` (RAW) | S |
| G3 | Camera model/make from video metadata for file and folder names | S |
| G4 | Also save HEIC/HEIF as JPEG during import | K |
| G5 | Sequence number per job code and per destination folder | S |
| G6 | Set metadata during import, as in Photo Mechanic (→ N16) | S |
| G7 | Handle camera file-number overflow (9999 → 0001) and files edited in-camera cleanly | S |
| G8 | Show RAW + JPEG as a single item (→ B23) | M |
| G9 | iOS Live Photos: link photo and video | K |
| G10 | Mark imported files read-only | K |
| G11 | Remember external source/destination and recognize it when the path reappears | S |
| G12 | Export/import settings (automatically satisfied with YAML, N25) | M |
| G13 | Ability to rename presets | M |
| G14 | Don't mount all local drives, only removable media | M |
