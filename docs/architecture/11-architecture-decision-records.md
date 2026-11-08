# 11. Architecture Decision Records

Part of the [architecture](README.md).

ADRs live under [`docs/adr/`](../adr/) in MADR format, numbered, immutable after
acceptance (a change means a new ADR that supersedes the old one).

| No. | Title | Status |
|---|---|---|
| [0001](../adr/0001-record-architecture-decisions.md) | Record architecture decisions as ADRs | accepted |
| [0002](../adr/0002-rust-and-slint.md) | Rust and Slint | accepted |
| [0003](../adr/0003-files-and-xmp-are-the-source-of-truth.md) | Files and XMP are the source of truth, per-machine index | accepted |
| [0004](../adr/0004-modular-architecture.md) | Modular architecture with kernel and static registration | accepted |
| [0005](../adr/0005-ports-and-adapters.md) | Ports & adapters, business logic without I/O | accepted |
| [0006](../adr/0006-deterministic-tests.md) | Deterministic tests, no retries | accepted |
| [0007](../adr/0007-reproducible-builds-with-nix.md) | Reproducible builds with a pinned toolchain and Nix | accepted |
| [0008](../adr/0008-development-by-ai-agents.md) | Development by AI agents: enforce rules by machine | accepted |
| [0009](../adr/0009-minimal-dependencies.md) | Minimal dependencies, only established libraries | accepted |
| [0010](../adr/0010-specification-and-implementation-in-sync.md) | Every feature is specified; specification and implementation stay in sync | accepted |
| [0011](../adr/0011-configuration-in-yaml.md) | Configuration in YAML | accepted |
| [0012](../adr/0012-raw-decoding-with-libraw.md) | RAW decoding with LibRaw | accepted |
| [0013](../adr/0013-immich-integration.md) | Immich integration (read-only) | accepted |
| [0014](../adr/0014-license-gpl-3-or-later.md) | License GPL-3.0-or-later | accepted |
| [0015](../adr/0015-metadata-in-dng-files.md) | Metadata in DNG files | accepted |
| [0016](../adr/0016-thiserror-in-every-library-layer.md) | `thiserror` in every library layer | accepted |
| [0017](../adr/0017-no-binary-files-in-the-repository.md) | No binary files in the repository | accepted, decision 9 partly superseded by 0019 |
| [0018](../adr/0018-github-and-github-actions.md) | Hosting on GitHub, CI with GitHub Actions | accepted |
| [0019](../adr/0019-review-screenshots-in-a-comment.md) | Review screenshots are linked from a comment | accepted |
| [0020](../adr/0020-import-execution-and-verified-copy.md) | Import execution, verified copy, `blake3` and `clap` | accepted |
| [0021](../adr/0021-own-exif-reader-for-the-capture-time.md) | Own EXIF reader for the capture time | accepted |
| [0022](../adr/0022-jiff-as-the-time-library.md) | `jiff` as the time library | accepted |
| [0023](../adr/0023-device-detection-by-polling-mounts.md) | Device detection by polling mounts | accepted |
| [0024](../adr/0024-line-based-guided-import.md) | A line-based guided import | accepted |
| [0025](../adr/0025-remembered-state-of-the-guided-import.md) | Remembered state of the guided import | accepted |
| [0026](../adr/0026-unicode-normalization-of-target-names.md) | Unicode normalization of target names | accepted |
| [0027](../adr/0027-test-data-fetch.md) | Test data fetch with the tools of the Nix shell | accepted |
| [0028](../adr/0028-libraw-build-and-reference-decode.md) | LibRaw build and reference decode, from the phase 0 prototype | accepted |
| [0029](../adr/0029-persisted-sequence-counters.md) | Persisted sequence counters | accepted |
| [0030](../adr/0030-backup-destinations.md) | Backup destinations: one read to several verified targets | accepted |
| [0031](../adr/0031-import-history-in-sqlite.md) | Import history in SQLite | accepted |
| [0032](../adr/0032-import-profiles-and-yaml-parser.md) | Import profiles and the YAML parser | accepted |
| [0033](../adr/0033-free-space-check.md) | Free-space check before an import | accepted |
| [0034](../adr/0034-cameras-through-libgphoto2.md) | Cameras and phones through libgphoto2 | accepted |
| [0035](../adr/0035-job-queue.md) | Job queue on a worker thread | accepted |
| [0036](../adr/0036-parallel-jobs-and-reservation-at-enqueue.md) | Parallel jobs and reservation at enqueue | accepted |
| [0037](../adr/0037-coverage-guided-fuzzing-with-cargo-fuzz.md) | Coverage-guided fuzzing with cargo-fuzz | accepted |
| [0038](../adr/0038-public-api-snapshots.md) | Public API snapshots with cargo-public-api on the stable toolchain | accepted |
| [0039](../adr/0039-xmp-parser-and-serializer.md) | XMP parser and serializer: quick-xml and an own model | accepted |
| [0040](../adr/0040-xmp-sidecars-written-by-the-import.md) | XMP sidecars written by the import | accepted |
| [0041](../adr/0041-sync-records-in-the-framehive-xmp-namespace.md) | Sync records in the Framehive XMP namespace (numbered 0037 until 2026-11-08) | accepted |
