// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

use std::fs;
use std::path::{Path, PathBuf};

use super::*;

const MANIFEST: &str = r#"[workspace]
members = [
    "crates/domain",
    "crates/kernel",
    "xtask",
]

[workspace.dependencies]
domain = { path = "crates/domain" }
kernel = { path = "crates/kernel" }
thiserror = { version = "2" }
"#;

/// A workspace in the system temporary directory, removed again on drop.
struct Workspace {
    root: PathBuf,
}

impl Workspace {
    fn new(test: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "framehive-xtask-new-module-{test}-{}",
            std::process::id()
        ));
        // A leftover from an aborted run would make the test depend on history.
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("Cargo.toml"), MANIFEST).unwrap();
        Self { root }
    }

    fn read(&self, relative: &str) -> String {
        fs::read_to_string(self.root.join(relative)).unwrap()
    }

    fn files(&self) -> Vec<String> {
        let mut files = Vec::new();
        collect_files(&self.root, &self.root, &mut files);
        files.sort();
        files
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn collect_files(root: &Path, directory: &Path, files: &mut Vec<String>) {
    for entry in fs::read_dir(directory).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            collect_files(root, &path, files);
        } else {
            files.push(slash_path(path.strip_prefix(root).unwrap()));
        }
    }
}

#[test]
fn generates_both_crates_of_the_module() {
    let workspace = Workspace::new("structure");
    let created = generate(&workspace.root, "map").unwrap();
    let expected = [
        "Cargo.toml",
        "modules/map/map-ui/Cargo.toml",
        "modules/map/map-ui/README.md",
        "modules/map/map-ui/src/lib.rs",
        "modules/map/map/Cargo.toml",
        "modules/map/map/README.md",
        "modules/map/map/src/lib.rs",
        "modules/map/map/src/view_model.rs",
        "modules/map/map/tests/conformance.rs",
    ];
    assert_eq!(workspace.files(), expected);
    assert_eq!(created.len(), expected.len() - 1);
}

#[test]
fn declares_the_layers_and_the_dependency_of_the_ui_on_its_module() {
    let workspace = Workspace::new("layers");
    generate(&workspace.root, "map").unwrap();
    let module = workspace.read("modules/map/map/Cargo.toml");
    assert!(module.contains("name = \"map\"\n"));
    assert!(module.contains("layer = \"module\"\n"));
    assert!(module.contains("kernel.workspace = true\n"));
    let ui = workspace.read("modules/map/map-ui/Cargo.toml");
    assert!(ui.contains("name = \"map-ui\"\n"));
    assert!(ui.contains("layer = \"module-ui\"\n"));
    assert!(ui.contains("\nmap.workspace = true\n"));
}

#[test]
fn readmes_have_the_sections_check_architecture_requires() {
    let workspace = Workspace::new("readme");
    generate(&workspace.root, "map").unwrap();
    for readme in ["modules/map/map/README.md", "modules/map/map-ui/README.md"] {
        let text = workspace.read(readme);
        for section in ["## Purpose", "## Public API"] {
            assert!(
                text.lines().any(|line| line == section),
                "{readme} lacks {section}"
            );
        }
    }
}

#[test]
fn every_file_has_an_spdx_header_and_no_placeholder() {
    let workspace = Workspace::new("spdx");
    for file in generate(&workspace.root, "map").unwrap() {
        let text = workspace.read(&slash_path(&file));
        // The expected header is data here, not the header of this file.
        // REUSE-IgnoreStart
        assert!(
            text.contains("SPDX-FileCopyrightText: 2026 Framehive contributors"),
            "{} lacks the copyright header",
            file.display()
        );
        assert!(text.contains("SPDX-License-Identifier: GPL-3.0-or-later"));
        // REUSE-IgnoreEnd
        assert!(!text.contains(ID_PLACEHOLDER), "{}", file.display());
    }
}

#[test]
fn crate_roots_forbid_unsafe_code() {
    let workspace = Workspace::new("unsafe");
    generate(&workspace.root, "map").unwrap();
    for root in [
        "modules/map/map/src/lib.rs",
        "modules/map/map-ui/src/lib.rs",
    ] {
        assert!(workspace.read(root).contains("#![forbid(unsafe_code)]"));
    }
}

#[test]
fn registers_both_crates_in_the_workspace() {
    let workspace = Workspace::new("register");
    generate(&workspace.root, "map").unwrap();
    let manifest = workspace.read("Cargo.toml");
    assert!(manifest.contains("    \"modules/map/map\",\n    \"modules/map/map-ui\",\n"));
    assert!(manifest.contains("map = { path = \"modules/map/map\" }\n"));
    assert!(manifest.contains("map-ui = { path = \"modules/map/map-ui\" }\n"));
}

#[test]
fn uses_the_id_with_underscores_as_crate_and_path_name() {
    let workspace = Workspace::new("underscore");
    generate(&workspace.root, "migrate_digikam").unwrap();
    let conformance =
        workspace.read("modules/migrate_digikam/migrate_digikam/tests/conformance.rs");
    assert!(conformance.contains("migrate_digikam::module_id()"));
    assert!(
        workspace
            .read("modules/migrate_digikam/migrate_digikam-ui/Cargo.toml")
            .contains("name = \"migrate_digikam-ui\"")
    );
}

#[test]
fn rejects_invalid_ids_without_writing() {
    let workspace = Workspace::new("invalid");
    for id in [
        "",
        "Map",
        "1map",
        "map-view",
        "map_",
        "a".repeat(33).as_str(),
    ] {
        assert!(
            generate(&workspace.root, id).is_err(),
            "{id:?} was accepted"
        );
    }
    assert_eq!(workspace.files(), ["Cargo.toml"]);
    assert_eq!(workspace.read("Cargo.toml"), MANIFEST);
}

#[test]
fn rejects_ids_rust_reserves() {
    let workspace = Workspace::new("reserved");
    for id in ["type", "std", "test"] {
        assert_eq!(
            generate(&workspace.root, id),
            Err(format!(
                "module id {id:?} is reserved by Rust and cannot name a crate"
            ))
        );
    }
    assert_eq!(workspace.files(), ["Cargo.toml"]);
}

#[test]
fn rejects_an_existing_module_directory_without_writing() {
    let workspace = Workspace::new("existing");
    fs::create_dir_all(workspace.root.join("modules/map")).unwrap();
    assert_eq!(
        generate(&workspace.root, "map"),
        Err("modules/map already exists".to_owned())
    );
    assert_eq!(workspace.read("Cargo.toml"), MANIFEST);
}

#[test]
fn rejects_a_second_generation_of_the_same_module() {
    let workspace = Workspace::new("twice");
    generate(&workspace.root, "map").unwrap();
    let manifest = workspace.read("Cargo.toml");
    assert!(generate(&workspace.root, "map").is_err());
    assert_eq!(workspace.read("Cargo.toml"), manifest);
}

#[test]
fn rejects_ids_that_name_an_existing_crate() {
    let workspace = Workspace::new("crate");
    for id in ["kernel", "xtask", "thiserror"] {
        assert_eq!(
            generate(&workspace.root, id),
            Err(format!("the workspace already has a crate named {id:?}"))
        );
    }
    assert_eq!(workspace.files(), ["Cargo.toml"]);
}

#[test]
fn rejects_ids_whose_library_name_an_existing_crate_already_has() {
    let workspace = Workspace::new("library");
    generate(&workspace.root, "map").unwrap();
    let manifest = workspace.read("Cargo.toml");
    // `map-ui` and `map_ui` both compile to the library `map_ui`.
    assert_eq!(
        generate(&workspace.root, "map_ui"),
        Err("the workspace already has a crate named \"map-ui\"".to_owned())
    );
    assert_eq!(workspace.read("Cargo.toml"), manifest);
}

#[test]
fn run_without_an_id_prints_the_usage() {
    let workspace = Workspace::new("usage");
    assert_eq!(
        run(&workspace.root, None),
        Err("usage: cargo xtask new-module <id>".to_owned())
    );
    assert_eq!(workspace.files(), ["Cargo.toml"]);
}
