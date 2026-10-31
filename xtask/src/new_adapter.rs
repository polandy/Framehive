// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! `cargo xtask new-adapter <port> <name>`: generates the adapter crate
//! `crates/adapters/<name>` (package `adapter-<name>`) from the template in
//! `xtask/templates/adapter/` and registers it in the workspace (architecture §3).
//!
//! `<port>` is the service crate whose port the adapter implements. The port `domain`
//! stands for the ports of `domain` itself, such as `Clock`: every adapter may depend
//! on `domain`, so the crate then declares no `port`.

use std::fs;
use std::path::{Path, PathBuf};

use crate::new_module::manifest::{self, Member};
use crate::new_module::{library_name, slash_path, update_lock_file, write_new};

const NAME_PLACEHOLDER: &str = "{{name}}";
const PORT_PLACEHOLDER: &str = "{{port}}";
/// Stands for the whole `port = "…"` line, including its line break.
const PORT_KEY_PLACEHOLDER: &str = "{{port_key}}\n";
const ADAPTERS_DIRECTORY: &str = "crates/adapters";
const PACKAGE_PREFIX: &str = "adapter-";
/// The crate whose ports need no `port` key, because every layer above it may use it.
const DOMAIN: &str = "domain";
const MAX_NAME_LENGTH: usize = 32;
const USAGE: &str = "usage: cargo xtask new-adapter <port> <name>";

/// A template file and where it goes, relative to the crate directory.
struct TemplateFile {
    path: &'static str,
    text: &'static str,
}

const TEMPLATE: [TemplateFile; 4] = [
    TemplateFile {
        path: "Cargo.toml",
        text: include_str!("../templates/adapter/Cargo.toml.template"),
    },
    TemplateFile {
        path: "README.md",
        text: include_str!("../templates/adapter/README.md.template"),
    },
    TemplateFile {
        path: "src/lib.rs",
        text: include_str!("../templates/adapter/src/lib.rs.template"),
    },
    TemplateFile {
        path: "tests/conformance.rs",
        text: include_str!("../templates/adapter/tests/conformance.rs.template"),
    },
];

/// Generates the adapter, refreshes `Cargo.lock` and prints the next steps.
pub(crate) fn run(root: &Path, port: Option<&str>, name: Option<&str>) -> Result<(), String> {
    let (Some(port), Some(name)) = (port, name) else {
        return Err(USAGE.to_owned());
    };
    let created = generate(root, port, name)?;
    update_lock_file(root)?;
    for path in &created {
        println!("created {}", path.display());
    }
    println!(
        "registered {PACKAGE_PREFIX}{name} in Cargo.toml\n\nnext steps (architecture §3):\n  \
         1. describe the adapter in {ADAPTERS_DIRECTORY}/{name}/README.md\n  \
         2. implement the port of {port} and run its conformance suite in tests/conformance.rs\n  \
         3. list any new dependency in dependencies.yaml (ADR 0009)\n  \
         4. run cargo xtask ci"
    );
    Ok(())
}

/// Writes the crate of adapter `name` for `port` below `root` and registers it in
/// `root/Cargo.toml`. Returns the created files, relative to `root`.
///
/// Every check runs before the first write, so a rejected argument leaves no trace.
pub(crate) fn generate(root: &Path, port: &str, name: &str) -> Result<Vec<PathBuf>, String> {
    let manifest_path = root.join("Cargo.toml");
    let manifest_text = fs::read_to_string(&manifest_path)
        .map_err(|error| format!("cannot read {}: {error}", manifest_path.display()))?;
    validate_name(name)?;
    let package = format!("{PACKAGE_PREFIX}{name}");
    let crate_directory = Path::new(ADAPTERS_DIRECTORY).join(name);
    reject_existing(root, &crate_directory, &manifest_text, &package)?;
    require_port(&manifest_text, port)?;

    let crate_path = slash_path(&crate_directory);
    let registered = manifest::register(
        &manifest_text,
        &[Member {
            name: &package,
            path: &crate_path,
        }],
    )?;

    let created = write_crate(root, &crate_directory, port, name)?;
    fs::write(&manifest_path, registered)
        .map_err(|error| format!("cannot write {}: {error}", manifest_path.display()))?;
    Ok(created)
}

/// Names are lowercase words of letters and digits joined by single hyphens, such
/// as `fs` or `gphoto2`; the package name adds the prefix `adapter-`.
fn validate_name(name: &str) -> Result<(), String> {
    let invalid = || {
        format!(
            "adapter name {name:?} must be at most {MAX_NAME_LENGTH} lowercase letters, \
             digits and single hyphens, starting with a letter"
        )
    };
    let starts_with_letter = name.chars().next().is_some_and(|c| c.is_ascii_lowercase());
    let words_are_valid = name.split('-').all(|word| {
        !word.is_empty()
            && word
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
    });
    if name.len() > MAX_NAME_LENGTH || !starts_with_letter || !words_are_valid {
        return Err(invalid());
    }
    Ok(())
}

fn reject_existing(
    root: &Path,
    crate_directory: &Path,
    manifest: &str,
    package: &str,
) -> Result<(), String> {
    if root.join(crate_directory).exists() {
        return Err(format!("{} already exists", slash_path(crate_directory)));
    }
    if let Some(existing) = manifest::known_crates(manifest)
        .iter()
        .find(|known| library_name(known) == library_name(package))
    {
        return Err(format!(
            "the workspace already has a crate named {existing:?}"
        ));
    }
    Ok(())
}

fn require_port(manifest: &str, port: &str) -> Result<(), String> {
    if manifest::known_crates(manifest)
        .iter()
        .any(|known| known == port)
    {
        return Ok(());
    }
    Err(format!(
        "the workspace has no crate named {port:?} to take the port from"
    ))
}

fn write_crate(
    root: &Path,
    crate_directory: &Path,
    port: &str,
    name: &str,
) -> Result<Vec<PathBuf>, String> {
    let port_key = if port == DOMAIN {
        String::new()
    } else {
        format!("port = \"{port}\"\n")
    };
    TEMPLATE
        .iter()
        .map(|file| {
            let relative = crate_directory.join(file.path);
            let text = file
                .text
                .replace(PORT_KEY_PLACEHOLDER, &port_key)
                .replace(NAME_PLACEHOLDER, name)
                .replace(PORT_PLACEHOLDER, port);
            write_new(&root.join(&relative), &text)?;
            Ok(relative)
        })
        .collect()
}

#[cfg(test)]
mod tests;
