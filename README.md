<!--
SPDX-FileCopyrightText: 2026 Framehive contributors
SPDX-License-Identifier: GPL-3.0-or-later
-->

# Framehive

A modular photo library for Linux and macOS, written in Rust with a Slint UI. Files
and their XMP sidecars are the source of truth; the index can always be rebuilt from
them.

Status: early development. Nothing is usable yet.

## Getting started

```sh
nix develop          # or: direnv allow
cargo xtask ci       # every check, exactly as CI runs it
```

## Documentation

| Topic | Document |
|---|---|
| What we build | [docs/requirements/](docs/requirements/README.md) |
| Feature behaviour | [specs/](specs/) |
| Architecture | [docs/architecture/](docs/architecture/README.md) |
| Decisions | [docs/adr/](docs/adr/) |
| Rules for contributors and AI agents | [CLAUDE.md](CLAUDE.md) |
| Changes | [CHANGELOG.md](CHANGELOG.md) |

## License

GPL-3.0-or-later, see [LICENSE](LICENSE) and ADR 0014.
