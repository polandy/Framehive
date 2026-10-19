// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Registers crates in the root `Cargo.toml`: the `members` list and the local entries
//! of `[workspace.dependencies]`.
//!
//! The edit is textual and touches only those two lists, so comments and the rest of
//! the manifest stay exactly as they are. It expects the layout of our own manifest:
//! one member per line, and the local path entries as one contiguous block.

const MEMBERS_START: &str = "members = [";
const LIST_END: &str = "]";
const DEPENDENCIES_HEADER: &str = "[workspace.dependencies]";
const INDENT: &str = "    ";

/// A crate to register: its package name and its directory relative to the root.
pub(crate) struct Member<'a> {
    pub(crate) name: &'a str,
    pub(crate) path: &'a str,
}

/// Returns the names of the crates the manifest already knows: the last component of
/// each member path and each key of `[workspace.dependencies]`.
pub(crate) fn known_crates(manifest: &str) -> Vec<String> {
    let lines: Vec<&str> = manifest.lines().collect();
    let members = members_range(&lines).map_or_else(Vec::new, |range| {
        lines[range]
            .iter()
            .filter_map(|line| quoted(line))
            .filter_map(|path| path.rsplit('/').next())
            .map(str::to_owned)
            .collect()
    });
    let dependencies = dependencies_range(&lines)
        .into_iter()
        .flat_map(|range| lines[range].iter())
        .filter_map(|line| dependency_key(line))
        .map(str::to_owned);
    members.into_iter().chain(dependencies).collect()
}

/// Adds every crate to `members` and as a path entry to `[workspace.dependencies]`.
/// Both lists stay sorted.
pub(crate) fn register(manifest: &str, crates: &[Member<'_>]) -> Result<String, String> {
    let mut lines: Vec<String> = manifest.lines().map(str::to_owned).collect();
    add_dependencies(&mut lines, crates)?;
    add_members(&mut lines, crates)?;
    let mut text = lines.join("\n");
    text.push('\n');
    Ok(text)
}

fn add_members(lines: &mut Vec<String>, crates: &[Member<'_>]) -> Result<(), String> {
    let range = members_range(&as_strs(lines)).ok_or("Cargo.toml has no members list")?;
    let mut entries: Vec<String> = lines[range.clone()].to_vec();
    entries.extend(
        crates
            .iter()
            .map(|member| format!("{INDENT}\"{}\",", member.path)),
    );
    entries.sort();
    lines.splice(range, entries);
    Ok(())
}

fn add_dependencies(lines: &mut Vec<String>, crates: &[Member<'_>]) -> Result<(), String> {
    let section = dependencies_range(&as_strs(lines))
        .ok_or_else(|| format!("Cargo.toml has no {DEPENDENCIES_HEADER} section"))?;
    let local: Vec<usize> = section
        .filter(|&index| lines[index].contains("{ path = \""))
        .collect();
    let (Some(&first), Some(&last)) = (local.first(), local.last()) else {
        return Err(format!("{DEPENDENCIES_HEADER} has no local path entries"));
    };
    if last - first + 1 != local.len() {
        return Err(format!(
            "the local path entries in {DEPENDENCIES_HEADER} must form one block"
        ));
    }
    let mut entries: Vec<String> = lines[first..=last].to_vec();
    entries.extend(
        crates
            .iter()
            .map(|member| format!("{} = {{ path = \"{}\" }}", member.name, member.path)),
    );
    entries.sort();
    lines.splice(first..=last, entries);
    Ok(())
}

/// The lines between `members = [` and the closing `]`.
fn members_range(lines: &[&str]) -> Option<std::ops::Range<usize>> {
    let start = lines.iter().position(|line| line.trim() == MEMBERS_START)? + 1;
    let length = lines[start..]
        .iter()
        .position(|line| line.trim() == LIST_END)?;
    Some(start..start + length)
}

/// The lines of `[workspace.dependencies]` up to the next section header.
fn dependencies_range(lines: &[&str]) -> Option<std::ops::Range<usize>> {
    let start = lines
        .iter()
        .position(|line| line.trim() == DEPENDENCIES_HEADER)?
        + 1;
    let length = lines[start..]
        .iter()
        .position(|line| line.trim_start().starts_with('['))
        .unwrap_or(lines.len() - start);
    Some(start..start + length)
}

fn dependency_key(line: &str) -> Option<&str> {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') {
        return None;
    }
    line.split_once('=').map(|(key, _)| key.trim())
}

fn quoted(line: &str) -> Option<&str> {
    let (_, rest) = line.split_once('"')?;
    rest.split_once('"').map(|(value, _)| value)
}

fn as_strs(lines: &[String]) -> Vec<&str> {
    lines.iter().map(String::as_str).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const MANIFEST: &str = r#"[workspace]
resolver = "3"
members = [
    "crates/domain",
    "crates/kernel",
    "xtask",
]

# Every direct dependency is pinned here.
[workspace.dependencies]
domain = { path = "crates/domain" }
kernel = { path = "crates/kernel" }
thiserror = { version = "2", default-features = false, features = ["std"] }

[workspace.lints.rust]
unsafe_code = "forbid"
"#;

    const MAP: [Member<'static>; 2] = [
        Member {
            name: "map",
            path: "modules/map/map",
        },
        Member {
            name: "map-ui",
            path: "modules/map/map-ui",
        },
    ];

    #[test]
    fn lists_members_and_dependency_keys() {
        assert_eq!(
            known_crates(MANIFEST),
            ["domain", "kernel", "xtask", "domain", "kernel", "thiserror"]
        );
    }

    #[test]
    fn adds_sorted_members_and_local_dependencies() {
        let registered = register(MANIFEST, &MAP).unwrap();
        assert!(registered.contains(
            "members = [
    \"crates/domain\",
    \"crates/kernel\",
    \"modules/map/map\",
    \"modules/map/map-ui\",
    \"xtask\",
]"
        ));
        assert!(registered.contains(
            "[workspace.dependencies]
domain = { path = \"crates/domain\" }
kernel = { path = \"crates/kernel\" }
map = { path = \"modules/map/map\" }
map-ui = { path = \"modules/map/map-ui\" }
thiserror = "
        ));
    }

    #[test]
    fn keeps_the_rest_of_the_manifest_unchanged() {
        let registered = register(MANIFEST, &MAP).unwrap();
        let removed: Vec<&str> = registered
            .lines()
            .filter(|line| !line.contains("modules/map/"))
            .collect();
        assert_eq!(removed, MANIFEST.lines().collect::<Vec<_>>());
    }

    #[test]
    fn rejects_a_manifest_without_members() {
        let manifest = MANIFEST.replace("members = [", "packages = [");
        assert_eq!(
            register(&manifest, &MAP),
            Err("Cargo.toml has no members list".to_owned())
        );
    }

    #[test]
    fn rejects_split_local_dependency_blocks() {
        let manifest = MANIFEST.replace("kernel = { path", "other = \"1\"\nkernel = { path");
        assert!(register(&manifest, &MAP).unwrap_err().contains("one block"));
    }
}
