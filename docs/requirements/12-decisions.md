# 12. Decisions

Part of the [requirements](README.md).

| Date | Decision |
|---|---|
| 2026-10-17 | Platforms: Linux first, macOS as the second primary platform, Windows undecided |
| 2026-10-17 | GUI: Slint |
| 2026-10-17 | MVP additionally includes: PTP/MTP (Q2), loupe + culling (N10), profiles (N8), headless CLI (N18) → all set to **M** |
| 2026-10-17 | Import runs as a background job; while one import is running, the next can be planned |
| 2026-10-17 | Framehive becomes a modular photo library (like digiKam); import is one module; no editor for now |
| 2026-10-17 | The source of truth is files + XMP; the database is a per-machine index |
| 2026-10-17 | Migration: digiKam database and folders with XMP |
| 2026-10-17 | Storage locations: local, NAS, external (including offline) drives, multiple machines (Linux + Mac) |
| 2026-10-17 | Early after v1: map, faces, duplicates & similarity, content search (all local) |
| 2026-10-17 | Architecture: strictly modular (modules isolated, communication via the kernel), ports & adapters, standards implemented exactly, deterministic tests with no retries, reproducible builds, all rules checked by tooling since AI agents do the development (architecture, ADR 0001–0008) |
| 2026-10-17 | Keep dependencies minimal; libraries only where it makes sense and only from established products; allowlist with rationale (ADR 0009) |
| 2026-10-17 | Every feature is specified in `specs/`; specification, tests and implementation always change together (ADR 0010) |
| 2026-10-17 | Human-editable configuration is YAML, designed to be expressive and easy to understand (ADR 0011) |
| 2026-10-17 | Everything in the repository is English: code, docs, specs, ADRs, commits, PRs – regardless of the language used with the agent |
| 2026-10-17 | Product name: **Framehive** |
| 2026-10-17 | RAW + JPEG grouping is an MVP feature (5.3); the Fujifilm X-T2 and X-T5 are reference cameras whose RAF/JPEG/HEIF files must work fully (2.8) |
| 2026-10-17 | Test data: the project owner's own X-T2/X-T5 photos, cleaned (no GPS, no serial number, no people), released as CC0, plus raw.pixls.us |
| 2026-10-17 | Contradicting sidecars within an asset: the most recently modified wins, with a notice (LIBRARY-001) |
| 2026-10-17 | Dual-slot RAW/JPEG separation is paired across separate import jobs via the import history (LIBRARY-001) |
| 2026-10-17 | License: GPL-3.0-or-later (ADR 0014) |
| 2026-10-17 | Nix flake for reproducible builds and the development environment (ADR 0007) |
| 2026-10-17 | YAML library: phase 0 evaluates `saphyr` first, libyaml as fallback (ADR 0011) |
| 2026-10-17 | Storage: Nextcloud sync folder is the main library root and the import target (6.4) |
| 2026-10-17 | Immich is used read-only: faces, people, albums, tags, favorites, places synced one-way into XMP; content search and previews live (6.5, ADR 0013) |
| 2026-10-17 | People module: Immich is the first face provider, local recognition comes later as a second provider |
| 2026-10-18 | The Canon EOS 7D is a reference camera (CR2, 2.8); DNG is a reference format (Adobe DNG Converter, drones, action cameras) |
| 2026-10-18 | DNG metadata goes to the sidecar; an option also embeds it into the DNG for Lightroom (ADR 0015) |
| 2026-10-18 | No binaries in the repository: test data in `framehive-testdata`, pinned in `testdata.lock`; screenshot tests store BLAKE3 hashes, diffs go to CI artifacts; review screenshots live in `framehive-assets` (ADR 0017) |
| 2026-10-18 | Hosted on GitHub; CI is one GitHub Actions workflow that runs `cargo xtask ci` on Linux and macOS (ADR 0018) |
| 2026-10-18 | Review screenshots for UI pull requests are linked from a PR comment, not from the description that becomes the squash commit (ADR 0019) |
| 2026-11-03 | ADRs 0004, 0007 and 0008 accepted; 0008 now requires only the README sections that are checked (Purpose, and Public API for libraries) |
| 2026-11-03 | ADRs 0012 and 0013 accepted; the LibRaw prototype validates 0012 and a failure means a superseding ADR; XMP namespace `https://polandy.github.io/framehive/ns/1.0/` (prefix `framehive`); `ring` as the TLS crypto backend |
| 2026-11-04 | The import module has priority; IMPORT-002 accepted: sequence counters in a state file reserved under a lock (ADR 0029), letters end at `zz`, no sharing between machines, a persisted counter cannot recognize files already imported until the import history (N3) |
| 2026-11-04 | Import history (N3, N4) accepted: SQLite through rusqlite `bundled` in the machine-local state folder (ADR 0031), recognized by name, size and capture time first and by checksum last, files deleted from the library stay imported before, resuming is running the import again, no override to import again yet (IMPORT-009, CLI-008) |
| 2026-11-08 | XMP milestone (N16), all 27 points with the recommended option: N16 at import plus a general XMP model (D1); source `.xmp` sidecars come with their file (D2); ADRs 0039 and 0040 (D3); default sidecar naming `original_with_extension` (D4); specs CORE-009, IMPORT-019 and IMPORT-020 (D5); seven PRs, layered then vertical (D6); `quick-xml` with an own model and serializer (D7); rewrite keeps every value, not the bytes (D8); canonical form without `<?xpacket?>` (D9); `xmp:MetadataDate` on every write (D10); job code, creator, copyright, keywords, no rating (D11, D12); global `metadata:` overridden by the profile (D13); IMMICH-001 keyword mapping (D14); `{year}` is the capture year (D15); a sidecar only when there are values (D16); photos only (D17); keywords unioned, job code replaces, creator and copyright kept (D18); a sidecar shares its file's conflict outcome (D19); a failed sidecar fails its file and keeps the card (D20); backups get the same sidecar (D21); no permission changes (D22); sidecars only, no embedding (D23); check before the rename (D24); exiftool in CI plus program fixtures (D25); fixtures exported by the owner, CC0 (D26); part of the import job (D27) |
