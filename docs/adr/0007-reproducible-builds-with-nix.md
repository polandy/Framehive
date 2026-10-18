# 0007 – Reproducible builds with a pinned toolchain and Nix

- Status: proposed
- Date: 2026-10-17

## Context

Builds and tests should run identically on Linux and macOS, locally and in CI.
Otherwise native dependencies (libgphoto2, ffmpeg, ONNX Runtime) would vary by
distribution and Homebrew state.

## Decision

- `rust-toolchain.toml` and `Cargo.lock` are checked in; all Cargo commands use
  `--locked`.
- The development environment and the CI environment come from a single Nix flake
  with `flake.lock` (Linux and macOS).
- `SOURCE_DATE_EPOCH`, `--remap-path-prefix`; CI checks bit-for-bit equality of the
  Linux release builds.
- `cargo xtask ci` is the single entry point and runs exactly what CI runs.

## Alternatives

- *Containers only (Docker)*: does not cover macOS.
- *Only a pinned Rust toolchain*: native dependencies stay uncontrolled.

## Consequences

- Nix must be installed on development machines (possible on Arch and macOS).
- End-user packages (Flatpak, AppImage, `.dmg`) are built from the same environment.
