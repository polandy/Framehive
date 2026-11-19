// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Part of `cargo xtask check-architecture`: every crate uses the workspace lints,
//! and an FFI adapter, which cannot inherit them, repeats them exactly
//! (architecture §7).

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use crate::workspace::Package;

/// The workspace lint that FFI adapters leave out, because they use `unsafe`.
const FFI_DROPPED: (&str, &str) = ("rust", "unsafe_code = \"forbid\"");

/// The lints FFI adapters add to guard their `unsafe` blocks.
const FFI_ADDED: [(&str, &str); 3] = [
    ("rust", "unsafe_op_in_unsafe_fn = \"deny\""),
    ("clippy", "undocumented_unsafe_blocks = \"deny\""),
    ("clippy", "multiple_unsafe_ops_per_block = \"deny\""),
];

/// Lint entries by table (`rust`, `clippy`), each a trimmed `key = value` line.
type Lints = BTreeMap<String, BTreeSet<String>>;

/// Returns every crate whose lints differ from the workspace lints.
pub(crate) fn check(root: &Path, packages: &[Package]) -> Result<Vec<String>, String> {
    let workspace = lint_tables(&read(&root.join("Cargo.toml"))?, "workspace.lints.");
    let ffi_lints = ffi_lints(&workspace);
    let mut problems = Vec::new();
    for package in packages {
        let manifest = read(&package.directory.join("Cargo.toml"))?;
        problems.extend(check_manifest(
            &package.name,
            package.ffi,
            &manifest,
            &ffi_lints,
        ));
    }
    Ok(problems)
}

fn read(path: &Path) -> Result<String, String> {
    fs::read_to_string(path).map_err(|error| format!("cannot read {}: {error}", path.display()))
}

/// The workspace lints as an FFI adapter must declare them.
fn ffi_lints(workspace: &Lints) -> Lints {
    let mut lints = workspace.clone();
    let (table, line) = FFI_DROPPED;
    if let Some(entries) = lints.get_mut(table) {
        entries.remove(line);
    }
    for (table, line) in FFI_ADDED {
        lints
            .entry(table.to_owned())
            .or_default()
            .insert(line.to_owned());
    }
    lints
}

fn check_manifest(name: &str, ffi: bool, manifest: &str, ffi_lints: &Lints) -> Vec<String> {
    if !ffi {
        return if inherits_workspace_lints(manifest) {
            Vec::new()
        } else {
            vec![format!(
                "{name}: Cargo.toml must set [lints] workspace = true"
            )]
        };
    }
    let declared = lint_tables(manifest, "lints.");
    let mut problems = Vec::new();
    for (table, line) in entries(ffi_lints) {
        if !declared
            .get(table)
            .is_some_and(|lines| lines.contains(line))
        {
            problems.push(format!("{name}: [lints.{table}] lacks `{line}`"));
        }
    }
    for (table, line) in entries(&declared) {
        if !ffi_lints
            .get(table)
            .is_some_and(|lines| lines.contains(line))
        {
            problems.push(format!(
                "{name}: [lints.{table}] has `{line}`, which the workspace lints lack"
            ));
        }
    }
    problems
}

fn entries(lints: &Lints) -> impl Iterator<Item = (&str, &str)> {
    lints.iter().flat_map(|(table, lines)| {
        lines
            .iter()
            .map(move |line| (table.as_str(), line.as_str()))
    })
}

fn inherits_workspace_lints(manifest: &str) -> bool {
    section_lines(manifest, "[lints]").any(|line| line == "workspace = true")
}

/// The lint tables whose headers start with `[<prefix>`, such as `[lints.clippy]`.
fn lint_tables(manifest: &str, prefix: &str) -> Lints {
    let mut lints = Lints::new();
    let mut table = None;
    for line in meaningful_lines(manifest) {
        if line.starts_with('[') {
            table = line
                .strip_prefix('[')
                .and_then(|header| header.strip_prefix(prefix))
                .and_then(|rest| rest.strip_suffix(']'))
                .map(str::to_owned);
        } else if let Some(table) = &table {
            lints
                .entry(table.clone())
                .or_default()
                .insert(line.to_owned());
        }
    }
    lints
}

fn section_lines<'a>(manifest: &'a str, header: &'a str) -> impl Iterator<Item = &'a str> {
    meaningful_lines(manifest)
        .skip_while(move |line| *line != header)
        .skip(1)
        .take_while(|line| !line.starts_with('['))
}

fn meaningful_lines(manifest: &str) -> impl Iterator<Item = &str> {
    manifest
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
}

#[cfg(test)]
mod tests {
    use super::*;

    const WORKSPACE: &str = "[workspace.lints.rust]\nunsafe_code = \"forbid\"\nmissing_docs = \"warn\"\n\n[workspace.lints.clippy]\ndbg_macro = \"warn\"\n\n[profile.release]\nlto = true\n";

    const FFI_MANIFEST: &str = "[package]\nname = \"x\"\n\n[lints.rust]\nmissing_docs = \"warn\"\nunsafe_op_in_unsafe_fn = \"deny\"\n\n[lints.clippy]\ndbg_macro = \"warn\"\nundocumented_unsafe_blocks = \"deny\"\nmultiple_unsafe_ops_per_block = \"deny\"\n";

    fn check_ffi(manifest: &str) -> Vec<String> {
        let workspace = lint_tables(WORKSPACE, "workspace.lints.");
        check_manifest("adapter-x", true, manifest, &ffi_lints(&workspace))
    }

    fn check_plain(manifest: &str) -> Vec<String> {
        check_manifest("x", false, manifest, &Lints::new())
    }

    #[test]
    fn accepts_crate_that_inherits_workspace_lints() {
        assert!(check_plain("[package]\nname = \"x\"\n\n[lints]\nworkspace = true\n").is_empty());
    }

    #[test]
    fn rejects_crate_without_workspace_lints() {
        assert_eq!(
            check_plain("[package]\nname = \"x\"\n"),
            ["x: Cargo.toml must set [lints] workspace = true"]
        );
    }

    #[test]
    fn accepts_ffi_adapter_with_workspace_lints_and_ffi_additions() {
        assert_eq!(check_ffi(FFI_MANIFEST), Vec::<String>::new());
    }

    #[test]
    fn rejects_ffi_adapter_that_lacks_a_workspace_lint() {
        let manifest = FFI_MANIFEST.replace("dbg_macro = \"warn\"\n", "");
        assert_eq!(
            check_ffi(&manifest),
            ["adapter-x: [lints.clippy] lacks `dbg_macro = \"warn\"`"]
        );
    }

    #[test]
    fn rejects_ffi_adapter_with_a_weaker_or_extra_lint() {
        let manifest = FFI_MANIFEST.replace("dbg_macro = \"warn\"", "dbg_macro = \"allow\"");
        assert_eq!(
            check_ffi(&manifest),
            [
                "adapter-x: [lints.clippy] lacks `dbg_macro = \"warn\"`",
                "adapter-x: [lints.clippy] has `dbg_macro = \"allow\"`, which the workspace lints lack",
            ]
        );
    }
}
