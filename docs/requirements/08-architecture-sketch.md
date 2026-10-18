# 8. Architecture sketch

Part of the [requirements](README.md).

Binding architecture, module system, standards, and development rules:
**[architecture](../architecture/README.md)** and the ADRs under [adr/](../adr/). The list below
is the high-level functional breakdown; the target structure of the repository is in
[architecture §12](../architecture/12-repository-structure.md).

Functional building blocks:

```
framehive/
├── crates/
│   ├── meta/       # Read/write EXIF/XMP/IPTC, XMP merge
│   ├── template/   # Template engine for file and folder names
│   ├── import/     # Scan, import plan, copy and verification engine
│   ├── devices/    # Trait DeviceSource; impls: mount (udisks2/udev, macOS DiskArbitration), gphoto2, iOS
│   ├── library/    # Roots, index, queries, albums, file watcher, offline volumes
│   ├── jobs/       # Persistent queue, scheduler, priorities
│   ├── store/      # SQLite: index, import history, hashes, counters
│   ├── previews/   # Thumbnail and preview generation + cache
│   ├── ml/         # Faces, CLIP, perceptual hash (ONNX Runtime via `ort`), optional
│   ├── geo/        # GPX, map tiles, offline reverse geocoding
│   ├── migrate/    # digiKam import, reading XMP folders
│   ├── cli/        # Headless frontend
│   └── gui/        # Slint frontend with modules
```

## Technical risks

| Risk | Approach |
|---|---|
| Thumbnail grid with 200,000 entries in Slint | Prototype in phase 0 (virtualized grid, images loaded asynchronously) |
| There is no map widget for Slint | Render tiles ourselves (OSM raster tiles, caching), prototype before phase 3 |
| Writing XMP in Rust: few mature libraries | `xmp_toolkit` (bindings to the Adobe XMP Toolkit, C++) or a custom writer for the needed scope; compatibility tests (9) |
| ONNX Runtime is a native dependency (conflicts with N23) | Build it as an optional feature/plugin; the core stays without it |
| RAF decoding: X-Trans demosaicing, X-T5 lossy compressed RAF, 40 MP performance | LibRaw via our own FFI (ADR 0012); prototype with the X-T2/X-T5 reference set in phase 0; LibRaw minimum version pinned by test |
| Writing XMP into DNG originals (ADR 0015) | Off by default; write to a temporary copy, re-read and verify it, then rename; raw image digest checked before and after |
| SQLite index with multiple machines | Index is never shared (S7); sharing happens only via files |

Import flow: **Scan → Plan → Execute → Verify**, as a job in `jobs`. The *plan*
(source → destination files, conflicts, space needed) is a pure data structure,
usable the same way by GUI and CLI, and easy to test.

Library flow: change in the UI → index updated immediately → XMP write as a job
(queued if the root is offline) → confirmed by the watcher.

## GUI toolkit

**Decision: Slint** (rationale: performant with a GPU renderer, freely styleable and
equally good-looking on Linux and macOS, stable since 1.0, pure Rust build with no
C++ toolchain; the GPL-3.0 or royalty-free license fits an open desktop app). The most
mature alternative would be Qt/QML via `cxx-qt`, which however needs a C++/Qt
toolchain.

| Option | Pros | Cons |
|---|---|---|
| **GTK4 + libadwaita** (gtk4-rs, Relm4) | Native on GNOME, virtualized `GridView`, mature, good accessibility | Only mediocre on macOS/Windows; feels foreign on KDE |
| **Slint** | Cross-platform, own DSL, good performance | Large thumbnail grid needs custom work; mind the licensing model |
| **iced** | Pure Rust, Elm architecture, cross-platform | Young ecosystem, accessibility still weak |
| **Tauri** (web UI) | Quick to make it look good, freely styleable | Webview dependency, thumbnails have to go through IPC |
