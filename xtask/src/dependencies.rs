// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! `cargo xtask check-dependencies`: every direct external dependency must be in the
//! allowlist, used only by allowed layers and crates, with allowed features, and
//! pinned in the workspace (ADR 0009).

use std::fs;

use crate::allowlist::AllowedDependency;
use crate::workspace::{Dependency, Package};

const DEFAULT_FEATURES: &str = "default";
const DEPENDENCY_SECTIONS: [&str; 3] = [
    "[dependencies]",
    "[dev-dependencies]",
    "[build-dependencies]",
];

/// Returns every violation of the allowlist.
pub(crate) fn check(packages: &[Package], allowlist: &[AllowedDependency]) -> Vec<String> {
    let mut problems = Vec::new();
    for package in packages {
        for dependency in package.dependencies.iter().filter(|d| !d.is_local) {
            match allowlist.iter().find(|entry| entry.name == dependency.name) {
                Some(entry) => problems.extend(check_use(package, dependency, entry)),
                None => problems.push(format!(
                    "{}: dependency {:?} is not in dependencies.yaml (ADR 0009)",
                    package.name, dependency.name
                )),
            }
        }
        problems.extend(check_workspace_pinning(package));
    }
    problems.extend(unused_entries(packages, allowlist));
    problems
}

fn check_use(package: &Package, dependency: &Dependency, entry: &AllowedDependency) -> Vec<String> {
    let mut problems = Vec::new();
    let layer = package.layer.as_deref().unwrap_or("<none>");
    if !entry.allowed_layers.iter().any(|allowed| allowed == layer) {
        problems.push(format!(
            "{}: {:?} is not allowed in layer {layer}",
            package.name, dependency.name
        ));
    }
    if !entry.allowed_crates.is_empty() && !entry.allowed_crates.contains(&package.name) {
        problems.push(format!(
            "{}: {:?} is only allowed in {}",
            package.name,
            dependency.name,
            entry.allowed_crates.join(", ")
        ));
    }
    let defaults = dependency
        .uses_default_features
        .then(|| DEFAULT_FEATURES.to_owned());
    for feature in dependency.features.iter().cloned().chain(defaults) {
        if !entry.features.contains(&feature) {
            problems.push(format!(
                "{}: feature {feature:?} of {:?} is not listed in dependencies.yaml",
                package.name, dependency.name
            ));
        }
    }
    problems
}

/// Versions are pinned once in `[workspace.dependencies]`; crates only refer to them.
fn check_workspace_pinning(package: &Package) -> Vec<String> {
    let path = package.directory.join("Cargo.toml");
    let Ok(manifest) = fs::read_to_string(&path) else {
        return Vec::new();
    };
    unpinned_dependencies(&manifest)
        .into_iter()
        .map(|name| {
            format!(
                "{}: dependency {name:?} must use `workspace = true` (ADR 0009)",
                package.name
            )
        })
        .collect()
}

fn unpinned_dependencies(manifest: &str) -> Vec<String> {
    let mut in_section = false;
    let mut unpinned = Vec::new();
    for line in manifest.lines().map(str::trim) {
        if line.starts_with('[') {
            in_section = DEPENDENCY_SECTIONS.contains(&line);
            continue;
        }
        if !in_section || line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        let pinned = key.ends_with(".workspace") || value.contains("workspace = true");
        if !pinned {
            unpinned.push(key.to_owned());
        }
    }
    unpinned
}

fn unused_entries(packages: &[Package], allowlist: &[AllowedDependency]) -> Vec<String> {
    allowlist
        .iter()
        .filter(|entry| {
            !packages
                .iter()
                .flat_map(|package| &package.dependencies)
                .any(|dependency| dependency.name == entry.name)
        })
        .map(|entry| {
            format!(
                "dependencies.yaml: {:?} is not used by any crate",
                entry.name
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    fn entry(name: &str, layers: &[&str], features: &[&str]) -> AllowedDependency {
        AllowedDependency {
            name: name.to_owned(),
            allowed_layers: layers.iter().map(|l| (*l).to_owned()).collect(),
            allowed_crates: Vec::new(),
            features: features.iter().map(|f| (*f).to_owned()).collect(),
        }
    }

    fn package(name: &str, layer: &str, dependency: &str, features: &[&str]) -> Package {
        Package {
            name: name.to_owned(),
            layer: Some(layer.to_owned()),
            port: None,
            ffi: false,
            directory: PathBuf::from("/nonexistent"),
            dependencies: vec![Dependency {
                name: dependency.to_owned(),
                kind: "normal".to_owned(),
                is_local: false,
                uses_default_features: false,
                features: features.iter().map(|f| (*f).to_owned()).collect(),
            }],
        }
    }

    #[test]
    fn accepts_allowed_use() {
        let allowlist = [entry("thiserror", &["kernel"], &["std"])];
        let packages = [package("kernel", "kernel", "thiserror", &["std"])];
        assert!(check(&packages, &allowlist).is_empty());
    }

    #[test]
    fn rejects_unlisted_dependency() {
        let packages = [package("kernel", "kernel", "left-pad", &[])];
        assert_eq!(
            check(&packages, &[]),
            ["kernel: dependency \"left-pad\" is not in dependencies.yaml (ADR 0009)"]
        );
    }

    #[test]
    fn rejects_wrong_layer_crate_and_feature() {
        let mut restricted = entry("ureq", &["adapter"], &[]);
        restricted.allowed_crates = vec!["adapter-immich-http".to_owned()];
        let packages = [package("catalog", "service", "ureq", &["json"])];
        let problems = check(&packages, &[restricted]);
        assert_eq!(problems.len(), 3, "{problems:?}");
    }

    #[test]
    fn default_features_must_be_listed() {
        let mut package = package("kernel", "kernel", "thiserror", &[]);
        package.dependencies[0].uses_default_features = true;
        let problems = check(&[package], &[entry("thiserror", &["kernel"], &["std"])]);
        assert_eq!(
            problems,
            ["kernel: feature \"default\" of \"thiserror\" is not listed in dependencies.yaml"]
        );
    }

    #[test]
    fn reports_unused_entry() {
        assert_eq!(
            check(&[], &[entry("serde", &["kernel"], &[])]),
            ["dependencies.yaml: \"serde\" is not used by any crate"]
        );
    }

    #[test]
    fn finds_dependencies_not_pinned_in_the_workspace() {
        let manifest = "[package]\nname = \"x\"\n\n[dependencies]\nthiserror.workspace = true\n\
                        kernel = { workspace = true }\nserde = \"1\"\n\n[lints]\nworkspace = true\n";
        assert_eq!(unpinned_dependencies(manifest), ["serde"]);
    }
}
