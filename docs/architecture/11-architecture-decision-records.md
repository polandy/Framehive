# 11. Architecture Decision Records

Part of the [architecture](README.md).

ADRs live under [`docs/adr/`](../adr/) in MADR format, numbered, immutable after
acceptance (a change means a new ADR that supersedes the old one).

| No. | Title | Status |
|---|---|---|
| [0001](../adr/0001-record-architecture-decisions.md) | Record architecture decisions as ADRs | accepted |
| [0002](../adr/0002-rust-and-slint.md) | Rust and Slint | accepted |
| [0003](../adr/0003-files-and-xmp-are-the-source-of-truth.md) | Files and XMP are the source of truth, per-machine index | accepted |
| [0004](../adr/0004-modular-architecture.md) | Modular architecture with kernel and static registration | proposed |
| [0005](../adr/0005-ports-and-adapters.md) | Ports & adapters, business logic without I/O | proposed |
| [0006](../adr/0006-deterministic-tests.md) | Deterministic tests, no retries | proposed |
| [0007](../adr/0007-reproducible-builds-with-nix.md) | Reproducible builds with a pinned toolchain and Nix | proposed |
| [0008](../adr/0008-development-by-ai-agents.md) | Development by AI agents: enforce rules by machine | proposed |
| [0009](../adr/0009-minimal-dependencies.md) | Minimal dependencies, only established libraries | accepted |
| [0010](../adr/0010-specification-and-implementation-in-sync.md) | Every feature is specified; specification and implementation stay in sync | accepted |
| [0011](../adr/0011-configuration-in-yaml.md) | Configuration in YAML | accepted |
| [0012](../adr/0012-raw-decoding-with-libraw.md) | RAW decoding with LibRaw | proposed |
| [0013](../adr/0013-immich-integration.md) | Immich integration (read-only) | proposed |
| [0014](../adr/0014-license-gpl-3-or-later.md) | License GPL-3.0-or-later | accepted |
| [0015](../adr/0015-metadata-in-dng-files.md) | Metadata in DNG files | accepted |
| [0016](../adr/0016-thiserror-in-every-library-layer.md) | `thiserror` in every library layer | accepted |
| [0017](../adr/0017-no-binary-files-in-the-repository.md) | No binary files in the repository | accepted, decision 9 partly superseded by 0019 |
| [0018](../adr/0018-github-and-github-actions.md) | Hosting on GitHub, CI with GitHub Actions | accepted |
| [0019](../adr/0019-review-screenshots-in-a-comment.md) | Review screenshots are linked from a comment | accepted |
