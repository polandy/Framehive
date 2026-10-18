# 0009 – Minimal dependencies, only established libraries

- Status: accepted
- Date: 2026-10-17

## Context

Every dependency costs ongoing maintenance: updates, security advisories, breaking
changes, abandoned projects, license review, longer builds and a larger supply-chain
attack surface. The Rust ecosystem makes it tempting to pull in crates for small
things; one crate often drags in dozens more. The project should stay maintainable
long-term with low effort.

## Decision

### 1. Principle

A library is used only when its benefit clearly outweighs the ongoing maintenance
cost. The default is: **the standard library or our own, well-tested code**.

### 2. When a library makes sense

A library is justified when at least one of the following applies:

- The problem is **large or domain-deep**, and a home-grown implementation would be
  error-prone: SQL database, GUI toolkit, image decoding, cryptographic hash
  functions, ML inference, XMP parsing.
- The problem is **security-critical**, and a vetted implementation is better than
  our own.
- It is a **de-facto standard** that readers of the code expect (e.g. `serde`), and
  our own code would be less understandable.

Our own code is preferred when the function is small, clearly specified and easy to
test (rule of thumb: under ~300 lines). Examples: template parser, path mapping,
simple command-line helpers, progress calculation, retry/backoff logic, file copying
with verification.

### 3. What "established" means

Every direct dependency must meet **all** of the following criteria:

| Criterion | Requirement |
|---|---|
| Origin | Backed by an organization or an established product (e.g. Slint GmbH, SQLite, Adobe, Microsoft, Rust project teams) or a widely used crate with several active maintainers |
| Adoption | Broad use in well-known projects (rule of thumb: top crate in its category, many dependent crates) |
| Maintenance | A release or commit within the last 12 months, security advisories fixed promptly |
| Stability | Version ≥ 1.0 or a declared de-facto standard with a stable API |
| License | Compatible (MIT, Apache-2.0, BSD, Zlib, MPL-2.0; GPL only after review), allowed in `deny.toml` |
| Scope | Usable with `default-features = false` and only the features we need; the transitive tree is reasonable |

For bindings to native libraries, the underlying product counts (e.g. libgphoto2,
LibRaw, ONNX Runtime). If there is no established binding crate, we write a thin,
own FFI layer in the respective adapter instead of trusting a poorly maintained
binding. We prefer calling external programs (ffmpeg, exiftool in tests) as a process
rather than linking them.

### 4. Process

- All direct dependencies are listed in an **allowlist**, `dependencies.yaml`, at the
  repository root, with an entry per dependency giving its purpose, a rationale per
  section 2, a check per section 3, the crates allowed to use it (which layers may use
  it) and the enabled features.
- `cargo xtask check-dependencies` fails if a crate uses a direct dependency that is
  not in the allowlist, or that is not allowed for its layer.
- The total number of transitive dependencies is logged in CI; an increase beyond a
  fixed threshold requires a justification in the commit.
- `cargo deny` checks licenses, security advisories, duplicate versions and allowed
  sources (crates.io only, no git dependencies).
- Versions are pinned centrally in the workspace (`[workspace.dependencies]`).
- **Native dependencies** (C/C++ libraries) and **new dependencies in `domain` or
  `kernel`** require their own ADR.
- Dependencies are reviewed regularly (quarterly); ones that are no longer needed or
  are abandoned are removed or replaced.
- The same rules apply to development dependencies (tests, benchmarks). Tools that
  only run as a program in CI (e.g. `cargo-deny`) are not dependencies of the code and
  are pinned via the Nix environment (ADR 0007).

### 5. Layers

| Layer | Allowed |
|---|---|
| `domain` | Standard library only, with rare exceptions for the smallest de-facto standards (e.g. `serde` derive); every exception requires an ADR |
| `kernel`, services, modules | Standard library and allowlist entries approved for the layer |
| Adapters | The library or binding they wrap. An external library is wrapped in **exactly one** adapter, never used across the codebase |
| Apps | Additionally, the GUI toolkit and the argument parser |

This keeps every library swappable, and a switch affects exactly one crate.

## First candidates (each entry still to be reviewed)

| Purpose | Candidate | Rationale | Note |
|---|---|---|---|
| GUI | `slint` | ADR 0002 | only in `*-ui`, `ui-kit`, `apps/gui` |
| Index database | `rusqlite` (with bundled SQLite) | SQLite is the most established embedded DB product | only `adapter-sqlite` |
| Checksum | `blake3` | official implementation by the BLAKE3 authors | |
| XMP | `xmp_toolkit` (Adobe XMP Toolkit SDK) | Adobe's reference implementation, maintained by Adobe / the Content Authenticity Initiative | native dependency → own ADR; alternative: our own reader/writer limited to the scope we need |
| Serialization, configuration | `serde` + a YAML library (chosen in ADR 0011) | de-facto standard | |
| Errors | `thiserror` | de-facto standard, no runtime cost | |
| Logging | `tracing`, `tracing-subscriber` | de-facto standard, Tokio project | |
| Time | `time` | established, timezone offsets | alternatively evaluate `jiff` |
| Command line | `clap` | de-facto standard | only `apps/cli` and the `cli` feature of modules |
| Image decoding | `image` (only the formats we need) | established, `image-rs` organization | |
| RAW (beyond the embedded preview) | LibRaw via our own FFI | established product | needed in the MVP for Fujifilm X-Trans RAF, see ADR 0012 |
| HTTP client (Immich) | `ureq` with `rustls` | widely used, synchronous; rustls is the established Rust TLS stack | only `adapter-immich-http`, ADR 0013 |
| JSON (Immich API) | `serde_json` | de-facto standard, serde project | only `adapter-immich-http` |
| SHA-1 (Immich checksum) | `sha1` (RustCrypto) | RustCrypto organization | only `adapter-immich-http`, for matching, not for security |
| Camera (PTP) | libgphoto2 via our own FFI | established product | own ADR |
| ML inference | ONNX Runtime (Microsoft) via `ort` | established product | optional, own ADR |
| Video | `ffmpeg` as an external program | established product | no linking |
| Tests | `proptest`, `insta`, `tempfile` | widely used | `dev-dependencies` only |

Deliberately **not** a library but written ourselves: template language, embedded-JPEG
extraction from TIFF-based RAWs, EXIF reading of the needed fields (TIFF/EXIF is
clearly specified and is fuzzed; decided as part of the metadata ADR), job scheduler
(standard library: threads and channels; an async runtime only if an adapter strictly
needs one), perceptual hash, file-watcher reconciliation via polling on a NAS.

## Consequences

- Fewer updates, fewer security advisories, a smaller attack surface, faster builds,
  a manageable license situation.
- More of our own code for small functions; per ADR 0006 this code must be especially
  well tested (property tests, fuzzing for parsers).
- Every library is wrapped behind an adapter and therefore swappable.
- Libraries mentioned in `docs/requirements/` (e.g. `nom-exif`, `rawler`, `image_hasher`,
  `sqlite-vec`, `usearch`, `criterion`) are only ideas and must go through this
  process.
