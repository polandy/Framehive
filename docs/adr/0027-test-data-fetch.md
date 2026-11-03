# 0027 – Test data fetch with the tools of the Nix shell

- Status: accepted
- Date: 2026-11-03

## Context

ADR 0017 keeps large test data out of the repository. `testdata.lock` pins every file
with its URL, size and BLAKE3 hash, and `cargo xtask testdata fetch` downloads,
verifies and unpacks them into the test data cache. ADR 0017 left open how the command
downloads and hashes: "Its dependencies go through ADR 0009 when the command is
implemented."

The command needs HTTPS, BLAKE3 and, for the archives of `framehive-testdata`
(`fujifilm-v1.tar.zst`), Zstandard and tar. As Rust crates in xtask these would be
`ureq` with `rustls` and `ring`, `blake3`, `zstd` (a binding to the C library) and
`tar`: four direct dependencies, two with native code, each allowed today in exactly
one other crate. xtask so far uses only `kernel` and the standard library.

`testdata.lock` is not configuration that a person edits by hand (ADR 0011); it is
written by whoever publishes a reference set and read only by xtask. The workspace has
no YAML library yet; xtask reads `dependencies.yaml` with its own strict subset reader.

## Decision

1. **xtask calls the tools of the Nix shell** through `std::process::Command`: `curl`
   to download, `b3sum` to hash, and GNU `tar` with `zstd` to unpack. The flake pins
   them, so every machine and CI use the same versions (ADR 0007). GNU tar is pinned
   on macOS too, so the archive behaves the same on both systems. A missing tool fails
   with a message that names `nix develop`. xtask gains no new Rust dependency.
2. **`testdata.lock` is line-based text.** The first line that is not blank or a
   comment (`#`) is the header `framehive-testdata-lock 1`, the format version. Every
   further line is one entry of four fields separated by whitespace:

   ```text
   <blake3> <size_bytes> <path> <url>
   ```

   - `blake3`: 64 lowercase hexadecimal digits, the hash of the downloaded file.
   - `size_bytes`: its size in bytes, a decimal number.
   - `path`: where it goes in the cache, relative, with `/` as separator, without `.`
     or `..` components. A path ending in `/` marks a `.tar.zst` archive that is
     unpacked into that directory; any other path is a single file stored there.
     Two entries may not have the same path, and no path may lie inside an archive
     directory.
   - `url`: an `https://` URL without whitespace.

   Unknown header versions, missing or extra fields and invalid values are errors that
   name the line. `cargo xtask ci` checks the file, so a broken lock fails before it is
   used.
3. **The cache** is `.testdata/` in the repository root, or `FRAMEHIVE_TESTDATA_DIR`
   (ADR 0017 §3). A file entry is up to date when the file has the pinned size and
   hash. An archive entry is up to date when its directory holds the stamp file
   `.framehive-testdata-blake3` with the pinned hash; otherwise the directory is
   removed and unpacked again, so no stale file survives a new version.
4. **Download, verify, then place.** A file is downloaded next to its target as
   `<name>.part`, its size and hash are checked, and only then is it renamed to the
   target or unpacked. A wrong size or hash deletes the download and fails with both
   values. There are no retries (CLAUDE.md rule 4); running the command again starts
   over.

## Alternatives

- **Rust crates in xtask** (`ureq`, `blake3`, `zstd`, `tar`): no external tools, but
  four new direct dependencies, two of them with native code needing their own ADR,
  for a developer command. Each would also have to be allowed in a second crate.
- **YAML with `schema_version`** (ADR 0011): consistent with the configuration files,
  but needs a YAML reader for a file that is a list of four fields, and a lock file is
  not configuration.
- **A shell script:** needs no Rust, but is harder to test, has no strict parser for
  the lock and runs differently in fish, bash and zsh.

## Consequences

- The dev shell gains `curl`, `b3sum`, `zstd` and `gnutar`. Outside `nix develop` the
  command fails with a clear message.
- `framehive-testdata` publishes each archive as `.tar.zst`, readable by GNU tar with
  `--zstd`.
- Tests of the command use `file://` URLs to files the test creates, never the network
  (ADR 0006). Only the lock file itself is restricted to `https://`.
- Should Framehive later need an HTTP client or Zstandard in a product crate, that is
  decided there; xtask is not a reason to add one.
