// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The workspace as seen by `cargo metadata`, reduced to what the checks need.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::json::{self, Json};

/// Layers from architecture §3, plus `tooling` for `xtask`.
pub(crate) const LAYERS: [&str; 8] = [
    "domain",
    "kernel",
    "service",
    "module",
    "module-ui",
    "adapter",
    "app",
    "tooling",
];

/// A crate of the workspace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Package {
    pub(crate) name: String,
    /// `None` if the crate does not declare `[package.metadata.framehive] layer`.
    pub(crate) layer: Option<String>,
    /// For adapters: the service crate whose port the adapter implements.
    pub(crate) port: Option<String>,
    /// Whether the crate is an FFI adapter that may use `unsafe`.
    pub(crate) ffi: bool,
    pub(crate) directory: PathBuf,
    pub(crate) dependencies: Vec<Dependency>,
}

/// A direct dependency declared in a crate's manifest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Dependency {
    pub(crate) name: String,
    /// `normal`, `dev` or `build`.
    pub(crate) kind: String,
    /// Whether it is a crate of this workspace (path dependency).
    pub(crate) is_local: bool,
    pub(crate) uses_default_features: bool,
    pub(crate) features: Vec<String>,
}

/// Loads the workspace crates via `cargo metadata`.
pub(crate) fn load(root: &Path) -> Result<Vec<Package>, String> {
    let output = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned()))
        .args(["metadata", "--format-version", "1", "--no-deps", "--locked"])
        .current_dir(root)
        .output()
        .map_err(|error| format!("cannot run cargo metadata: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "cargo metadata failed:\n{}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    let text = String::from_utf8(output.stdout).map_err(|e| e.to_string())?;
    parse_metadata(&json::parse(&text)?)
}

fn parse_metadata(metadata: &Json) -> Result<Vec<Package>, String> {
    let members: Vec<&str> = metadata
        .get("workspace_members")
        .items()
        .iter()
        .filter_map(Json::as_str)
        .collect();
    metadata
        .get("packages")
        .items()
        .iter()
        .filter(|package| {
            package
                .get("id")
                .as_str()
                .is_some_and(|id| members.contains(&id))
        })
        .map(parse_package)
        .collect()
}

fn parse_package(package: &Json) -> Result<Package, String> {
    let name = package.get("name").as_str().ok_or("package without name")?;
    let manifest = package
        .get("manifest_path")
        .as_str()
        .ok_or_else(|| format!("{name}: no manifest_path"))?;
    let directory = Path::new(manifest)
        .parent()
        .ok_or_else(|| format!("{name}: manifest has no parent directory"))?
        .to_path_buf();
    let framehive = package.get("metadata").get("framehive");
    Ok(Package {
        name: name.to_owned(),
        layer: framehive.get("layer").as_str().map(str::to_owned),
        port: framehive.get("port").as_str().map(str::to_owned),
        ffi: framehive.get("ffi").as_bool().unwrap_or(false),
        directory,
        dependencies: package
            .get("dependencies")
            .items()
            .iter()
            .map(parse_dependency)
            .collect(),
    })
}

fn parse_dependency(dependency: &Json) -> Dependency {
    let name = dependency
        .get("rename")
        .as_str()
        .or(dependency.get("name").as_str());
    Dependency {
        name: name.unwrap_or_default().to_owned(),
        kind: dependency
            .get("kind")
            .as_str()
            .unwrap_or("normal")
            .to_owned(),
        is_local: dependency.get("path").as_str().is_some(),
        uses_default_features: dependency
            .get("uses_default_features")
            .as_bool()
            .unwrap_or(true),
        features: dependency
            .get("features")
            .items()
            .iter()
            .filter_map(Json::as_str)
            .map(str::to_owned)
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_members_layers_and_dependencies() {
        let metadata = json::parse(
            r#"{
              "workspace_members": ["kernel 0.1.0 (path+file:///w/crates/kernel)"],
              "packages": [{
                "id": "kernel 0.1.0 (path+file:///w/crates/kernel)",
                "name": "kernel",
                "manifest_path": "/w/crates/kernel/Cargo.toml",
                "metadata": {"framehive": {"layer": "kernel"}},
                "dependencies": [
                  {"name": "domain", "kind": null, "path": "/w/crates/domain",
                   "uses_default_features": true, "features": []},
                  {"name": "thiserror", "kind": "dev",
                   "uses_default_features": false, "features": ["std"]}
                ]
              }, {
                "id": "other 1.0.0 (registry+https://github.com/rust-lang/crates.io-index)",
                "name": "other", "manifest_path": "/x/Cargo.toml", "dependencies": []
              }]
            }"#,
        )
        .unwrap();
        let packages = parse_metadata(&metadata).unwrap();
        assert_eq!(packages.len(), 1);
        let kernel = &packages[0];
        assert_eq!(kernel.layer.as_deref(), Some("kernel"));
        assert_eq!(kernel.directory, PathBuf::from("/w/crates/kernel"));
        assert_eq!(kernel.dependencies[0].kind, "normal");
        assert!(kernel.dependencies[0].is_local);
        assert_eq!(kernel.dependencies[1].kind, "dev");
        assert!(!kernel.dependencies[1].is_local);
        assert!(!kernel.dependencies[1].uses_default_features);
        assert_eq!(kernel.dependencies[1].features, ["std"]);
    }
}
