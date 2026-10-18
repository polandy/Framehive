# 2. Overview

Part of the [architecture](README.md).

```
┌──────────────────────────────── Apps (Composition Root) ────────────────────────────────┐
│   framehive-gui (Slint)                         framehive-cli (clap)                    │
│   wires adapters to ports, registers modules, starts the kernel                         │
└───────────────┬─────────────────────────────────────────────────────┬───────────────────┘
                │                                                     │
┌───────────────▼───── Modules (business, isolated from one another) ─▼───────────────────┐
│  import    library    map    people    immich    similarity    migrate-digikam   …      │
│  per module: <module> (business logic, use cases, ViewModels) + <module>-ui (Slint UI)  │
└───────────────┬─────────────────────────────────────────────────────────────────────────┘
                │ only via the kernel API and service ports
┌───────────────▼─────────────── Kernel & shared services ────────────────────────────────┐
│  kernel     module contract, registry, event bus, commands, settings                    │
│  catalog    index & queries over all assets (read model, port: CatalogStore)            │
│  metadata   XMP/EXIF/IPTC model, read/write/merge (port: MetadataCodec)                 │
│  volumes    storage locations, volume IDs, path mapping, online/offline                 │
│  jobs       queue, scheduler, priorities, resume (port: JobStore)                       │
│  previews   thumbnails & previews, cache (port: PreviewDecoder)                         │
│  naming     template language for file and folder names                                 │
└───────────────┬─────────────────────────────────────────────────────────────────────────┘
                │
┌───────────────▼───────────────── Domain (pure, no I/O) ─────────────────────────────────┐
│  domain     Asset, Sidecar, Volume, Rating, Pick, ColorLabel, Tag, Region, GeoPoint,    │
│             ContentHash, Clock/Id ports …                                               │
└─────────────────────────────────────────────────────────────────────────────────────────┘

┌──────────────────────── Adapters (implementations of the ports) ────────────────────────┐
│  adapter-sqlite   adapter-fs   adapter-gphoto2   adapter-mtp   adapter-udisks           │
│  adapter-macos-disks   adapter-xmp   adapter-exif   adapter-raw   adapter-ffmpeg        │
│  adapter-onnx   adapter-osm-tiles   adapter-digikam-db   adapter-immich-http            │
└─────────────────────────────────────────────────────────────────────────────────────────┘
```
