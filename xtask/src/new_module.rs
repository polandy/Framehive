// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! `cargo xtask new-module <id>`: generates the two crates of a module from the
//! template in `xtask/templates/module/` and registers them in the workspace
//! (architecture §4.1, §4.5).

pub(crate) mod manifest;

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

use kernel::ModuleId;

use manifest::Member;

/// Placeholder in the templates for the module id.
const ID_PLACEHOLDER: &str = "{{id}}";
const MODULES_DIRECTORY: &str = "modules";
const UI_SUFFIX: &str = "-ui";

/// A template file and where it goes, relative to the crate directory.
struct TemplateFile {
    path: &'static str,
    text: &'static str,
}

const MODULE_TEMPLATE: [TemplateFile; 5] = [
    TemplateFile {
        path: "Cargo.toml",
        text: include_str!("../templates/module/module/Cargo.toml.template"),
    },
    TemplateFile {
        path: "README.md",
        text: include_str!("../templates/module/module/README.md.template"),
    },
    TemplateFile {
        path: "src/lib.rs",
        text: include_str!("../templates/module/module/src/lib.rs.template"),
    },
    TemplateFile {
        path: "src/view_model.rs",
        text: include_str!("../templates/module/module/src/view_model.rs.template"),
    },
    TemplateFile {
        path: "tests/conformance.rs",
        text: include_str!("../templates/module/module/tests/conformance.rs.template"),
    },
];

const UI_TEMPLATE: [TemplateFile; 3] = [
    TemplateFile {
        path: "Cargo.toml",
        text: include_str!("../templates/module/ui/Cargo.toml.template"),
    },
    TemplateFile {
        path: "README.md",
        text: include_str!("../templates/module/ui/README.md.template"),
    },
    TemplateFile {
        path: "src/lib.rs",
        text: include_str!("../templates/module/ui/src/lib.rs.template"),
    },
];

/// Names Cargo refuses as package names: Rust keywords and the built-in crates.
const RESERVED_NAMES: [&str; 56] = [
    "abstract",
    "alloc",
    "as",
    "async",
    "await",
    "become",
    "box",
    "break",
    "const",
    "continue",
    "core",
    "crate",
    "do",
    "dyn",
    "else",
    "enum",
    "extern",
    "false",
    "final",
    "fn",
    "for",
    "gen",
    "if",
    "impl",
    "in",
    "let",
    "loop",
    "macro",
    "match",
    "mod",
    "move",
    "mut",
    "override",
    "priv",
    "proc_macro",
    "pub",
    "ref",
    "return",
    "self",
    "static",
    "std",
    "struct",
    "super",
    "test",
    "trait",
    "true",
    "try",
    "type",
    "typeof",
    "unsafe",
    "unsized",
    "use",
    "virtual",
    "where",
    "while",
    "yield",
];

/// Generates the module, refreshes `Cargo.lock` and prints the next steps.
pub(crate) fn run(root: &Path, id: Option<&str>) -> Result<(), String> {
    let id = id.ok_or("usage: cargo xtask new-module <id>")?;
    let created = generate(root, id)?;
    update_lock_file(root)?;
    for path in &created {
        println!("created {}", path.display());
    }
    println!(
        "registered {id} and {id}{UI_SUFFIX} in Cargo.toml\n\nnext steps (architecture §4.5):\n  \
         1. describe the responsibility and boundaries in modules/{id}/{id}/README.md\n  \
         2. replace ExampleViewModel with the module's ViewModels\n  \
         3. register the module in the apps\n  \
         4. run cargo xtask ci"
    );
    Ok(())
}

/// Writes both crates of module `id` below `root` and registers them in
/// `root/Cargo.toml`. Returns the created files, relative to `root`.
///
/// Every check runs before the first write, so a rejected id leaves no trace.
pub(crate) fn generate(root: &Path, id: &str) -> Result<Vec<PathBuf>, String> {
    let manifest_path = root.join("Cargo.toml");
    let manifest_text = fs::read_to_string(&manifest_path)
        .map_err(|error| format!("cannot read {}: {error}", manifest_path.display()))?;
    validate_id(id)?;
    let module_directory = Path::new(MODULES_DIRECTORY).join(id);
    reject_existing(root, &module_directory, &manifest_text, id)?;

    let module_crate = module_directory.join(id);
    let ui_crate = module_directory.join(format!("{id}{UI_SUFFIX}"));
    let module_path = slash_path(&module_crate);
    let ui_path = slash_path(&ui_crate);
    let ui_name = format!("{id}{UI_SUFFIX}");
    let registered = manifest::register(
        &manifest_text,
        &[
            Member {
                name: id,
                path: &module_path,
            },
            Member {
                name: &ui_name,
                path: &ui_path,
            },
        ],
    )?;

    let mut created = write_crate(root, &module_crate, &MODULE_TEMPLATE, id)?;
    created.extend(write_crate(root, &ui_crate, &UI_TEMPLATE, id)?);
    fs::write(&manifest_path, registered)
        .map_err(|error| format!("cannot write {}: {error}", manifest_path.display()))?;
    Ok(created)
}

fn validate_id(id: &str) -> Result<(), String> {
    ModuleId::new(id).map_err(|error| error.to_string())?;
    if RESERVED_NAMES.contains(&id) {
        return Err(format!(
            "module id {id:?} is reserved by Rust and cannot name a crate"
        ));
    }
    Ok(())
}

fn reject_existing(
    root: &Path,
    module_directory: &Path,
    manifest: &str,
    id: &str,
) -> Result<(), String> {
    if root.join(module_directory).exists() {
        return Err(format!("{} already exists", module_directory.display()));
    }
    let ui_name = format!("{id}{UI_SUFFIX}");
    let known = manifest::known_crates(manifest);
    for name in [id, ui_name.as_str()] {
        if let Some(existing) = known
            .iter()
            .find(|known| library_name(known) == library_name(name))
        {
            return Err(format!(
                "the workspace already has a crate named {existing:?}"
            ));
        }
    }
    Ok(())
}

/// Cargo turns `-` into `_` for the library name, so `map-ui` and `map_ui` collide.
pub(crate) fn library_name(crate_name: &str) -> String {
    crate_name.replace('-', "_")
}

fn write_crate(
    root: &Path,
    crate_directory: &Path,
    template: &[TemplateFile],
    id: &str,
) -> Result<Vec<PathBuf>, String> {
    template
        .iter()
        .map(|file| {
            let relative = crate_directory.join(file.path);
            write_new(
                &root.join(&relative),
                &file.text.replace(ID_PLACEHOLDER, id),
            )?;
            Ok(relative)
        })
        .collect()
}

/// Writes a file that must not exist yet, creating its parent directories.
pub(crate) fn write_new(path: &Path, text: &str) -> Result<(), String> {
    let error = |error: std::io::Error| format!("cannot write {}: {error}", path.display());
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(error)?;
    }
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .and_then(|mut file| file.write_all(text.as_bytes()))
        .map_err(error)
}

/// Cargo.toml uses forward slashes on every platform.
pub(crate) fn slash_path(path: &Path) -> String {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

/// Adds the new crates to `Cargo.lock` through Cargo, which keeps it reproducible;
/// it is never edited by hand (CLAUDE.md rule 5).
///
/// `cargo fetch` resolves only what `Cargo.toml` is missing and leaves every already
/// locked version untouched. `cargo update` must not be used here: with no package
/// given it re-resolves the whole workspace and can silently bump unrelated
/// dependencies to whatever newer, semver-compatible version happens to be cached.
pub(crate) fn update_lock_file(root: &Path) -> Result<(), String> {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned());
    let output = Command::new(cargo)
        .args(["fetch", "--offline"])
        .current_dir(root)
        .output()
        .map_err(|error| format!("cannot run cargo fetch: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "cargo fetch failed:\n{}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests;
