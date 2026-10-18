// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! `cargo xtask check-architecture`: layer rules, module isolation, `unsafe` and
//! crate READMEs (architecture §3, §7).

use std::fs;

use crate::workspace::{LAYERS, Package};

/// README sections every crate must have. Library layers also need "Public API".
const README_SECTIONS: [&str; 1] = ["## Purpose"];
const LIBRARY_README_SECTIONS: [&str; 1] = ["## Public API"];
const UI_KIT: &str = "ui-kit";
const FORBID_UNSAFE: &str = "#![forbid(unsafe_code)]";

/// Returns every violation found in the workspace.
pub(crate) fn check(packages: &[Package]) -> Vec<String> {
    let mut problems = Vec::new();
    for package in packages {
        let Some(layer) = valid_layer(package, &mut problems) else {
            continue;
        };
        for dependency in package.dependencies.iter().filter(|d| d.is_local) {
            let Some(target) = packages.iter().find(|p| p.name == dependency.name) else {
                continue;
            };
            let Some(target_layer) = target.layer.as_deref() else {
                continue;
            };
            if !may_depend(package, layer, target, target_layer) {
                problems.push(format!(
                    "{} ({layer}) must not depend on {} ({target_layer}), see architecture §3",
                    package.name, target.name
                ));
            }
        }
        problems.extend(check_unsafe(package, layer));
        problems.extend(check_readme(package, layer));
    }
    problems
}

fn valid_layer<'a>(package: &'a Package, problems: &mut Vec<String>) -> Option<&'a str> {
    match package.layer.as_deref() {
        Some(layer) if LAYERS.contains(&layer) => Some(layer),
        Some(layer) => {
            problems.push(format!("{}: unknown layer {layer:?}", package.name));
            None
        }
        None => {
            problems.push(format!(
                "{}: missing [package.metadata.framehive] layer",
                package.name
            ));
            None
        }
    }
}

/// The dependency table of architecture §3.
fn may_depend(from: &Package, layer: &str, to: &Package, to_layer: &str) -> bool {
    if to_layer == "tooling" || to_layer == "app" {
        return false;
    }
    if layer == "tooling" || layer == "app" {
        return true;
    }
    match (layer, to_layer) {
        (_, "domain") => layer != "domain",
        (_, "kernel") => layer != "domain" && layer != "kernel",
        ("service" | "module", "service") => true,
        ("adapter", "service") => from.port.as_deref() == Some(to.name.as_str()),
        ("module-ui", "module") => from.name.strip_suffix("-ui") == Some(to.name.as_str()),
        ("module-ui", "module-ui") => to.name == UI_KIT,
        _ => false,
    }
}

fn check_unsafe(package: &Package, layer: &str) -> Vec<String> {
    let may_use_unsafe = layer == "adapter" && package.ffi;
    if may_use_unsafe {
        return Vec::new();
    }
    crate_roots(package)
        .into_iter()
        .filter(|(_, source)| !source.contains(FORBID_UNSAFE))
        .map(|(path, _)| format!("{path}: missing {FORBID_UNSAFE}"))
        .collect()
}

fn crate_roots(package: &Package) -> Vec<(String, String)> {
    ["src/lib.rs", "src/main.rs"]
        .iter()
        .map(|relative| package.directory.join(relative))
        .filter_map(|path| {
            let source = fs::read_to_string(&path).ok()?;
            Some((path.display().to_string(), source))
        })
        .collect()
}

fn check_readme(package: &Package, layer: &str) -> Vec<String> {
    let path = package.directory.join("README.md");
    let Ok(readme) = fs::read_to_string(&path) else {
        return vec![format!("{}: missing README.md", package.name)];
    };
    let is_library = !matches!(layer, "app" | "tooling");
    README_SECTIONS
        .iter()
        .chain(LIBRARY_README_SECTIONS.iter().filter(|_| is_library))
        .filter(|section| !readme.lines().any(|line| line.trim_end() == **section))
        .map(|section| format!("{}: README.md lacks section {section:?}", package.name))
        .collect()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::workspace::Dependency;

    fn package(name: &str, layer: &str, dependencies: &[&str]) -> Package {
        Package {
            name: name.to_owned(),
            layer: Some(layer.to_owned()),
            port: None,
            ffi: false,
            // A directory that does not exist, so README and unsafe checks report it.
            directory: PathBuf::from("/nonexistent").join(name),
            dependencies: dependencies
                .iter()
                .map(|dependency| Dependency {
                    name: (*dependency).to_owned(),
                    kind: "normal".to_owned(),
                    is_local: true,
                    uses_default_features: true,
                    features: Vec::new(),
                })
                .collect(),
        }
    }

    fn layer_problems(packages: &[Package]) -> Vec<String> {
        check(packages)
            .into_iter()
            .filter(|problem| problem.contains("must not depend"))
            .collect()
    }

    #[test]
    fn allows_the_permitted_edges() {
        let packages = [
            package("domain", "domain", &[]),
            package("kernel", "kernel", &["domain"]),
            package("catalog", "service", &["domain", "kernel"]),
            package("import", "module", &["kernel", "catalog"]),
            package("import-ui", "module-ui", &["import", "ui-kit"]),
            package("ui-kit", "module-ui", &[]),
            package("gui", "app", &["import", "import-ui", "catalog"]),
        ];
        assert_eq!(layer_problems(&packages), Vec::<String>::new());
    }

    #[test]
    fn rejects_module_to_module() {
        let packages = [
            package("library", "module", &[]),
            package("import", "module", &["library"]),
        ];
        assert_eq!(
            layer_problems(&packages),
            ["import (module) must not depend on library (module), see architecture §3"]
        );
    }

    #[test]
    fn rejects_domain_to_kernel_and_ui_to_foreign_module() {
        let packages = [
            package("kernel", "kernel", &[]),
            package("domain", "domain", &["kernel"]),
            package("library", "module", &[]),
            package("import-ui", "module-ui", &["library"]),
        ];
        assert_eq!(layer_problems(&packages).len(), 2);
    }

    #[test]
    fn adapter_may_use_only_its_port() {
        let mut adapter = package("adapter-libraw", "adapter", &["previews", "catalog"]);
        adapter.port = Some("previews".to_owned());
        let packages = [
            package("previews", "service", &[]),
            package("catalog", "service", &[]),
            adapter,
        ];
        assert_eq!(
            layer_problems(&packages),
            ["adapter-libraw (adapter) must not depend on catalog (service), see architecture §3"]
        );
    }

    #[test]
    fn nothing_depends_on_apps_or_tooling() {
        let packages = [
            package("gui", "app", &[]),
            package("xtask", "tooling", &[]),
            package("import", "module", &["gui", "xtask"]),
        ];
        assert_eq!(layer_problems(&packages).len(), 2);
    }

    #[test]
    fn reports_missing_layer_and_readme() {
        let mut unlabeled = package("loose", "kernel", &[]);
        unlabeled.layer = None;
        let problems = check(&[unlabeled, package("kernel", "kernel", &[])]);
        assert!(problems.contains(&"loose: missing [package.metadata.framehive] layer".to_owned()));
        assert!(problems.contains(&"kernel: missing README.md".to_owned()));
    }
}
