# 7. Non-functional requirements

Part of the [requirements](README.md).

- **Performance**: scanning 2,000 files on a UHS-II card, first thumbnails < 2 s; the
  grid stays smooth with 20,000 files (virtualized list). Copy throughput close to the
  slowest device's limit; the source is read only once, destination and backups
  written in parallel.
- **Reliability**: no data loss on interruption (N2, N4); the source is never modified.
- **Library**: smooth with 200,000 images; filtering and search < 200 ms; app startup
  < 1 s; loupe preview from cache < 100 ms; keyboard tagging with no noticeable
  latency.
- **Resources**: thumbnail generation, hashing, and AI are limited to a configurable
  number of cores; memory stays bounded even with large libraries.
- **Accessibility**: keyboard operation, screen-reader labels, sufficient contrast.
- **License**: GPL-3.0-or-later (ADR 0014).
