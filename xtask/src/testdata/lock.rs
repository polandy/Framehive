// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Reader for `testdata.lock`, the pinned list of large test files (ADR 0017, ADR 0027).
//!
//! The format is line-based: a header with the format version, then one entry per
//! line, `<blake3> <size_bytes> <path> <url>`. Blank lines and `#` comments are
//! ignored.

const HEADER: &str = "framehive-testdata-lock";
const SUPPORTED_VERSION: &str = "1";
const HASH_LENGTH: usize = 64;

/// One pinned file or archive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Entry {
    /// BLAKE3 hash of the downloaded file, 64 lowercase hexadecimal digits.
    pub(crate) blake3: String,
    pub(crate) size_bytes: u64,
    /// Relative path in the cache; a trailing `/` marks a `.tar.zst` archive.
    pub(crate) path: String,
    pub(crate) url: String,
}

impl Entry {
    /// An archive is unpacked into the directory its path names.
    pub(crate) fn is_archive(&self) -> bool {
        self.path.ends_with('/')
    }
}

/// Parses the lock file. Errors name the line number.
pub(crate) fn parse(text: &str) -> Result<Vec<Entry>, String> {
    let mut header_seen = false;
    let mut entries: Vec<(usize, Entry)> = Vec::new();
    for (index, raw) in text.lines().enumerate() {
        let line_number = index + 1;
        let line = strip_comment(raw).trim();
        if line.is_empty() {
            continue;
        }
        let located = |message: String| format!("testdata.lock:{line_number}: {message}");
        if header_seen {
            entries.push((line_number, parse_entry(line).map_err(located)?));
        } else {
            check_header(line).map_err(located)?;
            header_seen = true;
        }
    }
    if !header_seen {
        return Err(format!(
            "testdata.lock: missing header \"{HEADER} {SUPPORTED_VERSION}\""
        ));
    }
    check_paths_are_distinct(&entries)?;
    Ok(entries.into_iter().map(|(_, entry)| entry).collect())
}

/// A comment starts at a `#` that begins the line or follows whitespace, so a `#`
/// inside a URL stays part of it.
fn strip_comment(line: &str) -> &str {
    let mut previous = ' ';
    for (index, character) in line.char_indices() {
        if character == '#' && previous.is_whitespace() {
            return &line[..index];
        }
        previous = character;
    }
    line
}

fn check_header(line: &str) -> Result<(), String> {
    match line.split_whitespace().collect::<Vec<_>>().as_slice() {
        [HEADER, SUPPORTED_VERSION] => Ok(()),
        [HEADER, version] => Err(format!(
            "unsupported format version {version:?}, expected {SUPPORTED_VERSION}"
        )),
        _ => Err(format!(
            "expected the header \"{HEADER} {SUPPORTED_VERSION}\""
        )),
    }
}

fn parse_entry(line: &str) -> Result<Entry, String> {
    let fields: Vec<&str> = line.split_whitespace().collect();
    let [blake3, size, path, url] = fields.as_slice() else {
        return Err(format!(
            "expected 4 fields (blake3 size_bytes path url), found {}",
            fields.len()
        ));
    };
    check_hash(blake3)?;
    check_path(path)?;
    check_url(url)?;
    Ok(Entry {
        blake3: (*blake3).to_owned(),
        size_bytes: parse_size(size)?,
        path: (*path).to_owned(),
        url: (*url).to_owned(),
    })
}

fn check_hash(hash: &str) -> Result<(), String> {
    let is_lower_hex = |c: char| c.is_ascii_digit() || ('a'..='f').contains(&c);
    if hash.len() == HASH_LENGTH && hash.chars().all(is_lower_hex) {
        Ok(())
    } else {
        Err(format!(
            "blake3 {hash:?} is not {HASH_LENGTH} lowercase hexadecimal digits"
        ))
    }
}

fn parse_size(size: &str) -> Result<u64, String> {
    // `u64::from_str` would also accept a leading `+`.
    if size.chars().all(|c| c.is_ascii_digit()) {
        size.parse()
            .map_err(|_| format!("size_bytes {size:?} is too large"))
    } else {
        Err(format!("size_bytes {size:?} is not a decimal number"))
    }
}

fn check_path(path: &str) -> Result<(), String> {
    let invalid = |reason: &str| Err(format!("path {path:?} {reason}"));
    if path.starts_with('/') {
        return invalid("must be relative");
    }
    if path.contains('\\') {
        return invalid("must use / as separator");
    }
    let components = path.strip_suffix('/').unwrap_or(path);
    for component in components.split('/') {
        match component {
            "" => return invalid("has an empty component"),
            "." | ".." => return invalid("must not contain . or .. components"),
            _ => {}
        }
    }
    Ok(())
}

fn check_url(url: &str) -> Result<(), String> {
    match url.strip_prefix("https://") {
        Some(rest) if !rest.is_empty() => Ok(()),
        _ => Err(format!("url {url:?} is not an https:// URL")),
    }
}

/// No two entries share a path, and no entry lies inside an archive directory.
fn check_paths_are_distinct(entries: &[(usize, Entry)]) -> Result<(), String> {
    for (position, (line_number, entry)) in entries.iter().enumerate() {
        for (earlier_line, earlier) in &entries[..position] {
            let clash = if entry.path == earlier.path {
                "the same path as"
            } else if lies_inside(&entry.path, earlier) || lies_inside(&earlier.path, entry) {
                "a path overlapping the archive in"
            } else {
                continue;
            };
            return Err(format!(
                "testdata.lock:{line_number}: path {:?} has {clash} line {earlier_line}",
                entry.path
            ));
        }
    }
    Ok(())
}

fn lies_inside(path: &str, archive: &Entry) -> bool {
    archive.is_archive()
        && (path.starts_with(&archive.path) || path == archive.path.trim_end_matches('/'))
}

#[cfg(test)]
mod tests;
