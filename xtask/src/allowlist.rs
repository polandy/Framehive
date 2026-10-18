// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Reader for `dependencies.yaml`, the allowlist of direct dependencies (ADR 0009).
//!
//! It accepts only the small YAML subset that the allowlist uses: top-level scalar
//! keys, one list of mappings, plain or double-quoted scalars and flow lists of plain
//! scalars. It is replaced by the YAML adapter once ADR 0011's library evaluation is
//! done.

/// One allowed direct dependency.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct AllowedDependency {
    pub(crate) name: String,
    pub(crate) allowed_layers: Vec<String>,
    /// If non-empty, only these crates may use the dependency.
    pub(crate) allowed_crates: Vec<String>,
    /// Cargo features that may be enabled; `default` stands for the default features.
    pub(crate) features: Vec<String>,
}

const SUPPORTED_SCHEMA_VERSION: &str = "1";
const REQUIRED_KEYS: [&str; 6] = [
    "name",
    "purpose",
    "rationale",
    "review",
    "allowed_layers",
    "features",
];
const OPTIONAL_KEYS: [&str; 2] = ["adr", "allowed_crates"];

/// Parses the allowlist. Errors name the line number.
pub(crate) fn parse(text: &str) -> Result<Vec<AllowedDependency>, String> {
    let mut schema_version = None;
    let mut in_list = false;
    let mut entries: Vec<Vec<(String, String)>> = Vec::new();
    for (index, raw) in text.lines().enumerate() {
        let line_number = index + 1;
        let line = strip_comment(raw);
        if line.trim().is_empty() {
            continue;
        }
        let error = |message: &str| format!("dependencies.yaml:{line_number}: {message}");
        if !line.starts_with(' ') {
            let (key, value) = split_key_value(line).ok_or_else(|| error("expected key: value"))?;
            match key {
                "schema_version" => schema_version = Some(value.to_owned()),
                "dependencies" if value.is_empty() => in_list = true,
                "dependencies" if value == "[]" => in_list = false,
                _ => return Err(error(&format!("unknown top-level key {key:?}"))),
            }
        } else if in_list {
            let entry_line = parse_entry_line(line, &mut entries).map_err(|m| error(&m))?;
            entries
                .last_mut()
                .ok_or_else(|| error("expected '- ' before the first key"))?
                .push(entry_line);
        } else {
            return Err(error("unexpected indentation"));
        }
    }
    if schema_version.as_deref() != Some(SUPPORTED_SCHEMA_VERSION) {
        return Err(format!(
            "dependencies.yaml: schema_version must be {SUPPORTED_SCHEMA_VERSION}"
        ));
    }
    entries
        .into_iter()
        .map(|pairs| build_entry(&pairs))
        .collect()
}

fn parse_entry_line(
    line: &str,
    entries: &mut Vec<Vec<(String, String)>>,
) -> Result<(String, String), String> {
    let trimmed = line.trim_start();
    let body = if let Some(rest) = trimmed.strip_prefix("- ") {
        entries.push(Vec::new());
        rest
    } else {
        trimmed
    };
    let (key, value) = split_key_value(body).ok_or("expected key: value")?;
    Ok((key.to_owned(), unquote(value)?))
}

fn build_entry(pairs: &[(String, String)]) -> Result<AllowedDependency, String> {
    let name = pairs
        .iter()
        .find(|(key, _)| key == "name")
        .map_or_else(|| "<unnamed>".to_owned(), |(_, value)| value.clone());
    let context = |message: String| format!("dependencies.yaml: entry {name:?}: {message}");
    for (key, _) in pairs {
        if !REQUIRED_KEYS.contains(&key.as_str()) && !OPTIONAL_KEYS.contains(&key.as_str()) {
            return Err(context(format!("unknown key {key:?}")));
        }
    }
    for key in REQUIRED_KEYS {
        if !pairs.iter().any(|(present, _)| present == key) {
            return Err(context(format!("missing key {key:?}")));
        }
    }
    let list = |key: &str| -> Result<Vec<String>, String> {
        pairs
            .iter()
            .find(|(present, _)| present == key)
            .map_or(Ok(Vec::new()), |(_, value)| parse_flow_list(value))
            .map_err(context)
    };
    Ok(AllowedDependency {
        name: name.clone(),
        allowed_layers: list("allowed_layers")?,
        allowed_crates: list("allowed_crates")?,
        features: list("features")?,
    })
}

fn strip_comment(line: &str) -> &str {
    let mut in_quotes = false;
    for (index, character) in line.char_indices() {
        match character {
            '"' => in_quotes = !in_quotes,
            '#' if !in_quotes && (index == 0 || line[..index].ends_with(' ')) => {
                return line[..index].trim_end();
            }
            _ => {}
        }
    }
    line.trim_end()
}

fn split_key_value(text: &str) -> Option<(&str, &str)> {
    let (key, value) = text.split_once(':')?;
    let key = key.trim();
    let valid_key = !key.is_empty()
        && key
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_');
    valid_key.then(|| (key, value.trim()))
}

fn unquote(value: &str) -> Result<String, String> {
    match value.strip_prefix('"') {
        Some(rest) => rest
            .strip_suffix('"')
            .filter(|inner| !inner.contains('"') && !inner.contains('\\'))
            .map(str::to_owned)
            .ok_or_else(|| format!("unsupported quoted string {value}")),
        None => Ok(value.to_owned()),
    }
}

fn parse_flow_list(value: &str) -> Result<Vec<String>, String> {
    let inner = value
        .strip_prefix('[')
        .and_then(|rest| rest.strip_suffix(']'))
        .ok_or_else(|| format!("expected a list like [a, b], got {value:?}"))?;
    Ok(inner
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(str::to_owned)
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID: &str = r#"
# comment
schema_version: 1
dependencies:
  - name: thiserror   # trailing comment
    purpose: Errors
    rationale: "Standard: yes"
    review: ok
    adr: "0016"
    allowed_layers: [domain, kernel]
    features: [std]
  - name: other
    purpose: p
    rationale: r
    review: r
    allowed_layers: [adapter]
    allowed_crates: [adapter-x]
    features: []
"#;

    #[test]
    fn parses_entries() {
        let entries = parse(VALID).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].name, "thiserror");
        assert_eq!(entries[0].allowed_layers, ["domain", "kernel"]);
        assert_eq!(entries[0].features, ["std"]);
        assert!(entries[0].allowed_crates.is_empty());
        assert_eq!(entries[1].allowed_crates, ["adapter-x"]);
        assert!(entries[1].features.is_empty());
    }

    #[test]
    fn accepts_empty_list() {
        assert!(
            parse("schema_version: 1\ndependencies: []\n")
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn rejects_unknown_key() {
        let text = VALID.replace("review: ok", "reviewed: ok");
        assert!(
            parse(&text)
                .unwrap_err()
                .contains("unknown key \"reviewed\"")
        );
    }

    #[test]
    fn rejects_missing_key() {
        let text = VALID.replace("    purpose: Errors\n", "");
        assert!(
            parse(&text)
                .unwrap_err()
                .contains("missing key \"purpose\"")
        );
    }

    #[test]
    fn rejects_wrong_schema_version() {
        let text = VALID.replace("schema_version: 1", "schema_version: 2");
        assert!(parse(&text).unwrap_err().contains("schema_version"));
    }

    #[test]
    fn rejects_unknown_top_level_key() {
        let error = parse("schema_version: 1\nextra: x\n").unwrap_err();
        assert!(error.contains("dependencies.yaml:2"), "{error}");
    }
}
