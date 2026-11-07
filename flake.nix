# SPDX-FileCopyrightText: 2026 Framehive contributors
# SPDX-License-Identifier: GPL-3.0-or-later

# Reproducible development shell and package (CLAUDE.md rule 5). The Rust toolchain
# comes from rust-toolchain.toml, so `nix develop`, `nix build` and CI use exactly the
# same compiler.
{
  description = "Framehive – modular photo library";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    { nixpkgs, rust-overlay, ... }:
    let
      toolchain = pkgs: pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ];
      forEachSystem =
        function:
        nixpkgs.lib.genAttrs systems (
          system:
          function (
            import nixpkgs {
              inherit system;
              overlays = [ rust-overlay.overlays.default ];
            }
          )
        );
      # LibRaw for adapter-libraw. nixpkgs builds it without zlib and libjpeg, so it
      # cannot decode deflate or lossy DNG, and on macOS without OpenMP, because Apple's
      # clang has no OpenMP runtime (ADR 0028 §2). OpenMP is propagated, so that
      # adapter-libraw links the same runtime.
      libraw =
        pkgs:
        pkgs.libraw.overrideAttrs (old: {
          buildInputs = (old.buildInputs or [ ]) ++ [
            pkgs.zlib
            pkgs.libjpeg
          ];
          propagatedBuildInputs =
            (old.propagatedBuildInputs or [ ])
            ++ pkgs.lib.optionals pkgs.stdenv.hostPlatform.isDarwin [ pkgs.llvmPackages.openmp ];
        });
      # libgphoto2 for adapter-gphoto2. The patch lets its `directory` driver read part
      # of a file, so the adapter's tests on a `disk:` port take the same chunked read
      # path as a PTP camera (ADR 0034 §3).
      libgphoto2 =
        pkgs:
        pkgs.libgphoto2.overrideAttrs (old: {
          patches = (old.patches or [ ]) ++ [ ./nix/patches/libgphoto2-directory-read-file.patch ];
        });
      # The `framehive` command from `framehive-cli`. Cargo needs the manifests and
      # sources of every workspace member, but nothing else, so a change to the
      # documentation does not rebuild the package.
      framehive =
        pkgs:
        let
          inherit (pkgs) lib;
          rustPlatform = pkgs.makeRustPlatform {
            cargo = toolchain pkgs;
            rustc = toolchain pkgs;
          };
          markdown = lib.fileset.fileFilter (file: file.hasExt "md") ./.;
        in
        rustPlatform.buildRustPackage {
          pname = "framehive";
          version = (lib.importTOML ./Cargo.toml).workspace.package.version;
          src = lib.fileset.toSource {
            root = ./.;
            fileset = lib.fileset.difference (lib.fileset.unions [
              ./Cargo.toml
              ./Cargo.lock
              ./.cargo
              ./apps
              ./crates
              ./modules
              ./xtask
            ]) markdown;
          };
          cargoLock.lockFile = ./Cargo.lock;
          cargoBuildFlags = [
            "-p"
            "framehive-cli"
          ];
          cargoTestFlags = [
            "-p"
            "framehive-cli"
          ];
          # `framehive import camera:` reads cameras through adapter-gphoto2 (ADR 0034).
          # The patched library also lets the package's tests read a `disk:` port.
          buildInputs = [ (libgphoto2 pkgs) ];
          meta = {
            description = "Modular photo library: imports photos and videos from the command line";
            license = lib.licenses.gpl3Plus;
            mainProgram = "framehive";
          };
        };
    in
    {
      devShells = forEachSystem (pkgs: {
        default = pkgs.mkShell {
          buildInputs = [
            (libraw pkgs)
            (libgphoto2 pkgs)
          ];
          packages = [
            (toolchain pkgs)
            pkgs.b3sum
            pkgs.cargo-deny
            pkgs.cargo-nextest
            pkgs.curl
            pkgs.gh
            pkgs.gnutar
            pkgs.reuse
            pkgs.zstd
          ];
        };
        # `nix develop .#fuzz`: cargo-fuzz needs unstable sanitizer and coverage flags,
        # so this shell has a nightly toolchain pinned to a date. The default shell, CI
        # and the release build keep the stable one (ADR 0037 §3). libFuzzer is built
        # from source by `libfuzzer-sys` with the C++ compiler of the shell.
        fuzz = pkgs.mkShell {
          packages = [
            pkgs.rust-bin.nightly."2026-10-15".minimal
            pkgs.cargo-fuzz
          ];
        };
      });

      packages = forEachSystem (pkgs: {
        default = framehive pkgs;
      });

      apps = forEachSystem (pkgs: {
        default = {
          type = "app";
          program = "${framehive pkgs}/bin/framehive";
        };
      });

      formatter = forEachSystem (pkgs: pkgs.nixfmt-rfc-style);
    };
}
