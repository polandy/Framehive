// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

use super::{Entry, parse};

const HASH: &str = "af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262";
const OTHER_HASH: &str = "0000000000000000000000000000000000000000000000000000000000000000";

fn lock(entries: &[&str]) -> String {
    let mut text = String::from("# pinned test data\nframehive-testdata-lock 1\n\n");
    for entry in entries {
        text.push_str(entry);
        text.push('\n');
    }
    text
}

fn error_of(text: &str) -> String {
    parse(text).expect_err("the lock must be rejected")
}

#[test]
fn parses_files_and_archives() {
    let text = lock(&[
        &format!("{HASH}  1024  dng/sample.dng  https://raw.pixls.us/sample.dng  # comment"),
        &format!("{OTHER_HASH} 0 fujifilm/ https://example.org/fujifilm-v1.tar.zst"),
    ]);
    let entries = parse(&text).expect("valid lock");
    assert_eq!(
        entries,
        vec![
            Entry {
                blake3: HASH.to_owned(),
                size_bytes: 1024,
                path: "dng/sample.dng".to_owned(),
                url: "https://raw.pixls.us/sample.dng".to_owned(),
            },
            Entry {
                blake3: OTHER_HASH.to_owned(),
                size_bytes: 0,
                path: "fujifilm/".to_owned(),
                url: "https://example.org/fujifilm-v1.tar.zst".to_owned(),
            },
        ]
    );
    assert!(!entries[0].is_archive());
    assert!(entries[1].is_archive());
}

#[test]
fn accepts_a_lock_without_entries() {
    assert_eq!(parse(&lock(&[])), Ok(Vec::new()));
}

#[test]
fn rejects_a_missing_header() {
    assert!(error_of("# nothing\n").contains("missing header"));
    let entry_first = format!("{HASH} 1 a https://x/a\n");
    assert!(error_of(&entry_first).contains("testdata.lock:1: expected the header"));
}

#[test]
fn rejects_an_unknown_format_version() {
    let error = error_of("framehive-testdata-lock 2\n");
    assert!(
        error.contains("unsupported format version \"2\""),
        "{error}"
    );
}

#[test]
fn rejects_a_wrong_number_of_fields() {
    let error = error_of(&lock(&[&format!("{HASH} 1 a")]));
    assert!(
        error.contains("testdata.lock:4: expected 4 fields"),
        "{error}"
    );
    let error = error_of(&lock(&[&format!("{HASH} 1 a https://x/a extra")]));
    assert!(error.contains("found 5"), "{error}");
}

#[test]
fn rejects_invalid_hashes() {
    let short = &HASH[..63];
    let upper = HASH.to_uppercase();
    for hash in [short, upper.as_str()] {
        let error = error_of(&lock(&[&format!("{hash} 1 a https://x/a")]));
        assert!(error.contains("64 lowercase hexadecimal digits"), "{error}");
    }
}

#[test]
fn rejects_invalid_sizes() {
    for size in ["-1", "+1", "1.5", "99999999999999999999"] {
        let error = error_of(&lock(&[&format!("{HASH} {size} a https://x/a")]));
        assert!(error.contains("size_bytes"), "{size}: {error}");
    }
}

#[test]
fn rejects_unsafe_paths() {
    for (path, reason) in [
        ("/etc/a", "must be relative"),
        ("a\\b", "must use / as separator"),
        ("a//b", "empty component"),
        ("../a", ". or .."),
        ("a/./b", ". or .."),
        ("/", "must be relative"),
    ] {
        let error = error_of(&lock(&[&format!("{HASH} 1 {path} https://x/a")]));
        assert!(error.contains(reason), "{path}: {error}");
    }
}

#[test]
fn rejects_urls_other_than_https() {
    for url in ["http://x/a", "file:///tmp/a", "https://"] {
        let error = error_of(&lock(&[&format!("{HASH} 1 a {url}")]));
        assert!(error.contains("is not an https:// URL"), "{url}: {error}");
    }
}

#[test]
fn rejects_duplicate_paths() {
    let error = error_of(&lock(&[
        &format!("{HASH} 1 dng/a.dng https://x/a"),
        &format!("{OTHER_HASH} 2 dng/a.dng https://x/b"),
    ]));
    assert!(
        error.contains("testdata.lock:5: path \"dng/a.dng\" has the same path as line 4"),
        "{error}"
    );
}

#[test]
fn rejects_a_file_inside_an_archive_directory() {
    let archive = format!("{HASH} 1 fujifilm/ https://x/f.tar.zst");
    let inside = format!("{OTHER_HASH} 2 fujifilm/a.raf https://x/a.raf");
    for entries in [[&archive, &inside], [&inside, &archive]] {
        let error = error_of(&lock(&[entries[0], entries[1]]));
        assert!(error.contains("overlapping the archive"), "{error}");
    }
}

#[test]
fn keeps_a_hash_sign_inside_a_url() {
    let text = lock(&[&format!("{HASH} 1 a https://x/a#part # comment")]);
    let entries = parse(&text).expect("valid lock");
    assert_eq!(entries[0].url, "https://x/a#part");
}

#[test]
fn rejects_a_file_with_the_name_of_an_archive_directory() {
    let error = error_of(&lock(&[
        &format!("{HASH} 1 fujifilm/ https://x/f.tar.zst"),
        &format!("{OTHER_HASH} 2 fujifilm https://x/f"),
    ]));
    assert!(error.contains("overlapping the archive"), "{error}");
}
