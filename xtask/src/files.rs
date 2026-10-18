// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! `cargo xtask check-files`: the repository contains only text files of moderate size
//! (ADR 0017). Large test data lives in `framehive-testdata`. Source files and
//! documents also stay small enough to be read in one go (architecture §7).

use std::fs;
use std::path::Path;
use std::process::Command;

/// Larger text files are almost always generated output or data dumps.
const MAX_FILE_BYTES: usize = 256 * 1024;

/// A longer Rust file mixes concerns and has to be split (architecture §7).
const MAX_RUST_LINES: usize = 400;

/// A longer document is split into one file per section, so a reader opens only the
/// part a change touches (architecture §7).
const MAX_DOCUMENT_BYTES: usize = 10 * 1024;

/// Documents exempt from the size limit: ADRs are immutable records, and the changelog
/// is read from the top.
const UNLIMITED_DOCUMENTS: [&str; 2] = ["docs/adr/", "CHANGELOG.md"];

/// Checks every file Git tracks or would track (untracked but not ignored).
pub(crate) fn check_repository(root: &Path) -> Result<Vec<String>, String> {
    let mut problems = Vec::new();
    for file in repository_files(root)? {
        // Tracked files deleted in the working tree have no content to check.
        let Ok(content) = fs::read(root.join(&file)) else {
            continue;
        };
        problems.extend(check_content(&file, &content));
    }
    Ok(problems)
}

fn repository_files(root: &Path) -> Result<Vec<String>, String> {
    let output = Command::new("git")
        .args([
            "ls-files",
            "-z",
            "--cached",
            "--others",
            "--exclude-standard",
        ])
        .current_dir(root)
        .output()
        .map_err(|error| format!("cannot run git: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "git ls-files failed:\n{}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    let listing = String::from_utf8(output.stdout).map_err(|error| error.to_string())?;
    Ok(listing
        .split('\0')
        .filter(|file| !file.is_empty())
        .map(str::to_owned)
        .collect())
}

/// Returns why `content` may not be in the repository, if it may not.
fn check_content(file: &str, content: &[u8]) -> Option<String> {
    let Ok(text) = std::str::from_utf8(content) else {
        return Some(binary_file(file));
    };
    if text.contains('\0') {
        return Some(binary_file(file));
    }
    if content.len() > MAX_FILE_BYTES {
        return Some(format!(
            "{file}: {} bytes, more than the limit of {MAX_FILE_BYTES} bytes (ADR 0017)",
            content.len()
        ));
    }
    check_size(file, text)
}

fn binary_file(file: &str) -> String {
    format!(
        "{file}: binary file; large test data belongs in framehive-testdata, small binary \
         fixtures are built by the test (ADR 0017)"
    )
}

/// Applies the per-kind limits for Rust sources and documents.
fn check_size(file: &str, text: &str) -> Option<String> {
    let lines = text.lines().count();
    if has_extension(file, "rs") && lines > MAX_RUST_LINES {
        return Some(format!(
            "{file}: {lines} lines, more than {MAX_RUST_LINES}; split it by concern \
             (architecture §7)"
        ));
    }
    let limited = !UNLIMITED_DOCUMENTS
        .iter()
        .any(|prefix| file.starts_with(prefix));
    if has_extension(file, "md") && limited && text.len() > MAX_DOCUMENT_BYTES {
        return Some(format!(
            "{file}: {} bytes, more than {MAX_DOCUMENT_BYTES}; split it into one file per \
             section with a README.md index (architecture §7)",
            text.len()
        ));
    }
    None
}

fn has_extension(file: &str, extension: &str) -> bool {
    Path::new(file)
        .extension()
        .is_some_and(|found| found.eq_ignore_ascii_case(extension))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_text() {
        assert_eq!(
            check_content("a.md", "# Title – ünïcode\n".as_bytes()),
            None
        );
    }

    #[test]
    fn rejects_nul_bytes() {
        let problem = check_content("a.bin", b"abc\0def").expect("rejected");
        assert!(problem.starts_with("a.bin: binary file"), "{problem}");
    }

    #[test]
    fn rejects_invalid_utf8() {
        // The start of a JPEG file.
        assert!(check_content("a.jpg", &[0xFF, 0xD8, 0xFF, 0xE0]).is_some());
    }

    #[test]
    fn rejects_large_text() {
        let content = vec![b'a'; MAX_FILE_BYTES + 1];
        let problem = check_content("dump.txt", &content).expect("rejected");
        assert!(problem.contains("more than the limit"), "{problem}");
    }

    #[test]
    fn rejects_long_rust_file() {
        let content = "// line\n".repeat(MAX_RUST_LINES + 1);
        let problem = check_content("src/big.rs", content.as_bytes()).expect("rejected");
        assert!(problem.contains("split it by concern"), "{problem}");
        let fitting = "// line\n".repeat(MAX_RUST_LINES);
        assert_eq!(check_content("src/ok.rs", fitting.as_bytes()), None);
    }

    #[test]
    fn rejects_long_document_outside_adrs_and_changelog() {
        let content = "a".repeat(MAX_DOCUMENT_BYTES + 1);
        let problem = check_content("docs/guide.md", content.as_bytes()).expect("rejected");
        assert!(problem.contains("one file per section"), "{problem}");
        assert_eq!(
            check_content("docs/adr/0001-x.md", content.as_bytes()),
            None
        );
        assert_eq!(check_content("CHANGELOG.md", content.as_bytes()), None);
    }
}
