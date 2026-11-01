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
          packages = [
            (toolchain pkgs)
            pkgs.cargo-deny
            pkgs.cargo-nextest
            pkgs.gh
            pkgs.reuse
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
