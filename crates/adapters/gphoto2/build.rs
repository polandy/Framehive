// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Compiles `src/shim.c` with the C compiler of the Nix shell and links libgphoto2. It
//! calls the compiler directly, so no build dependency is needed (ADR 0009, ADR 0034
//! §3).

// Cargo reads a build script's instructions from its standard output.
#![allow(clippy::print_stdout)]

use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

const SHIM: &str = "src/shim.c";
const SHIM_LIBRARY: &str = "framehive_gphoto2_shim";

fn main() -> Result<(), String> {
    println!("cargo:rerun-if-changed={SHIM}");
    println!("cargo:rerun-if-env-changed=CC");
    println!("cargo:rerun-if-env-changed=AR");
    let out_dir = PathBuf::from(env::var("OUT_DIR").map_err(|error| error.to_string())?);
    let object = out_dir.join("shim.o");
    run(Command::new(tool("CC", "cc"))
        .args(["-c", "-O2", "-fPIC", "-Wall", "-Werror", SHIM, "-o"])
        .arg(&object))?;
    let archive = out_dir.join(format!("lib{SHIM_LIBRARY}.a"));
    run(Command::new(tool("AR", "ar"))
        .arg("crs")
        .arg(&archive)
        .arg(&object))?;
    println!("cargo:rustc-link-search=native={}", out_dir.display());
    println!("cargo:rustc-link-lib=static={SHIM_LIBRARY}");
    println!("cargo:rustc-link-lib=dylib=gphoto2");
    println!("cargo:rustc-link-lib=dylib=gphoto2_port");
    Ok(())
}

/// The program named by the environment variable `variable`, or `fallback`.
fn tool(variable: &str, fallback: &str) -> PathBuf {
    env::var_os(variable).map_or_else(|| Path::new(fallback).to_path_buf(), PathBuf::from)
}

fn run(command: &mut Command) -> Result<(), String> {
    let status = command
        .status()
        .map_err(|error| format!("cannot run {command:?}: {error}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{command:?} failed with {status}"))
    }
}
