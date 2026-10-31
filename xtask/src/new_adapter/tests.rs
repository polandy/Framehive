// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

use std::fs;
use std::path::{Path, PathBuf};

use super::*;

const MANIFEST: &str = r#"[workspace]
members = [
    "crates/domain",
    "crates/services/files",
    "xtask",
]

[workspace.dependencies]
domain = { path = "crates/domain" }
files = { path = "crates/services/files" }
thiserror = { version = "2" }
"#;

/// A workspace in the system temporary directory, removed again on drop.
struct Workspace {
    root: PathBuf,
}

impl Workspace {
    fn new(test: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "framehive-xtask-new-adapter-{test}-{}",
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
fn generates_the_adapter_crate() {
    let workspace = Workspace::new("structure");
    let created = generate(&workspace.root, "files", "fs").unwrap();
    let expected = [
        "Cargo.toml",
        "crates/adapters/fs/Cargo.toml",
        "crates/adapters/fs/README.md",
        "crates/adapters/fs/src/lib.rs",
        "crates/adapters/fs/tests/conformance.rs",
    ];
    assert_eq!(workspace.files(), expected);
    assert_eq!(created.len(), expected.len() - 1);
}

#[test]
fn declares_the_adapter_layer_its_port_and_the_dependency_on_it() {
    let workspace = Workspace::new("layer");
    generate(&workspace.root, "files", "fs").unwrap();
    let manifest = workspace.read("crates/adapters/fs/Cargo.toml");
    assert!(manifest.contains("name = \"adapter-fs\"\n"));
    assert!(manifest.contains("layer = \"adapter\"\nport = \"files\"\n\n[dependencies]\n"));
    assert!(manifest.contains("\nfiles.workspace = true\n"));
}

#[test]
fn an_adapter_for_a_port_of_domain_declares_no_port() {
    let workspace = Workspace::new("domain");
    generate(&workspace.root, "domain", "clock").unwrap();
    let manifest = workspace.read("crates/adapters/clock/Cargo.toml");
    assert!(manifest.contains("layer = \"adapter\"\n\n[dependencies]\n"));
    assert!(!manifest.contains("port ="));
    assert!(manifest.contains("\ndomain.workspace = true\n"));
}

#[test]
fn readme_has_the_sections_check_architecture_requires() {
    let workspace = Workspace::new("readme");
    generate(&workspace.root, "files", "fs").unwrap();
    let text = workspace.read("crates/adapters/fs/README.md");
    for section in ["## Purpose", "## Public API"] {
        assert!(text.lines().any(|line| line == section), "lacks {section}");
    }
}

#[test]
fn every_file_has_an_spdx_header_and_no_placeholder() {
    let workspace = Workspace::new("spdx");
    for file in generate(&workspace.root, "files", "fs").unwrap() {
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
        assert!(!text.contains("{{"), "{}", file.display());
    }
}

#[test]
fn crate_root_forbids_unsafe_code() {
    let workspace = Workspace::new("unsafe");
    generate(&workspace.root, "files", "fs").unwrap();
    assert!(
        workspace
            .read("crates/adapters/fs/src/lib.rs")
            .contains("#![forbid(unsafe_code)]")
    );
}

#[test]
fn registers_the_crate_in_the_workspace() {
    let workspace = Workspace::new("register");
    generate(&workspace.root, "files", "fs").unwrap();
    let manifest = workspace.read("Cargo.toml");
    assert!(
        manifest.contains("members = [\n    \"crates/adapters/fs\",\n    \"crates/domain\",\n")
    );
    assert!(manifest.contains("adapter-fs = { path = \"crates/adapters/fs\" }\n"));
}

#[test]
fn rejects_invalid_names_without_writing() {
    let workspace = Workspace::new("invalid");
    for name in [
        "",
        "Fs",
        "1fs",
        "fs_std",
        "fs-",
        "-fs",
        "fs--std",
        "a".repeat(33).as_str(),
    ] {
        assert!(
            generate(&workspace.root, "files", name).is_err(),
            "{name:?} was accepted"
        );
    }
    assert_eq!(workspace.files(), ["Cargo.toml"]);
    assert_eq!(workspace.read("Cargo.toml"), MANIFEST);
}

#[test]
fn rejects_an_unknown_port_without_writing() {
    let workspace = Workspace::new("port");
    assert_eq!(
        generate(&workspace.root, "previews", "libraw"),
        Err("the workspace has no crate named \"previews\" to take the port from".to_owned())
    );
    assert_eq!(workspace.files(), ["Cargo.toml"]);
}

#[test]
fn rejects_an_existing_crate_directory_without_writing() {
    let workspace = Workspace::new("existing");
    fs::create_dir_all(workspace.root.join("crates/adapters/fs")).unwrap();
    assert_eq!(
        generate(&workspace.root, "files", "fs"),
        Err("crates/adapters/fs already exists".to_owned())
    );
    assert_eq!(workspace.read("Cargo.toml"), MANIFEST);
}

#[test]
fn rejects_a_second_generation_of_the_same_adapter() {
    let workspace = Workspace::new("twice");
    generate(&workspace.root, "files", "fs").unwrap();
    let manifest = workspace.read("Cargo.toml");
    assert!(generate(&workspace.root, "files", "fs").is_err());
    assert_eq!(workspace.read("Cargo.toml"), manifest);
}

#[test]
fn rejects_a_package_name_the_workspace_already_has() {
    let workspace = Workspace::new("crate");
    let manifest = MANIFEST.replace(
        "domain = {",
        "adapter_fs = { path = \"elsewhere\" }\ndomain = {",
    );
    fs::write(workspace.root.join("Cargo.toml"), &manifest).unwrap();
    assert_eq!(
        generate(&workspace.root, "files", "fs"),
        Err("the workspace already has a crate named \"adapter_fs\"".to_owned())
    );
    assert_eq!(workspace.files(), ["Cargo.toml"]);
}

#[test]
fn run_without_both_arguments_prints_the_usage() {
    let workspace = Workspace::new("usage");
    for (port, name) in [(None, None), (Some("files"), None)] {
        assert_eq!(run(&workspace.root, port, name), Err(USAGE.to_owned()));
    }
    assert_eq!(workspace.files(), ["Cargo.toml"]);
}
