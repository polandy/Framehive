// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

#![allow(clippy::unwrap_used, reason = "tests may panic on bad fixtures")]

use std::io::BufRead;
use std::path::{Path, PathBuf};

use adapter_xmp::QuickXml;
use domain::testing::ManualClock;
use domain::{Timestamp, UtcOffset};
use files::FileSystem;
use files::testing::{FnvHasher, MemoryFileSystem, MemoryTrash};
use metadata::xmp::{SidecarFiles, SidecarNaming};

use super::*;

const FOLDER: &str = "/photos";
const MARKED_ARRAY: &str = r#"<x:xmpmeta xmlns:x="adobe:ns:meta/"><rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"><rdf:Description rdf:about="" xmlns:framehive="https://polandy.github.io/framehive/ns/1.0/"><framehive:KeepRaw><rdf:Bag><rdf:li>True</rdf:li></rdf:Bag></framehive:KeepRaw></rdf:Description></rdf:RDF></x:xmpmeta>"#;
const EMPTY_SIDECAR: &str = r#"<x:xmpmeta xmlns:x="adobe:ns:meta/"><rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"><rdf:Description rdf:about=""/></rdf:RDF></x:xmpmeta>"#;

/// A library folder in memory, with a trash that can refuse files.
struct Library {
    files: MemoryFileSystem,
    trash: MemoryTrash,
    hasher: FnvHasher,
    xml: QuickXml,
    clock: ManualClock,
}

/// What a command wrote, and its exit code.
struct Ran {
    exit_code: u8,
    output: String,
    errors: String,
}

impl Library {
    fn new() -> Self {
        let files = MemoryFileSystem::new();
        files.add_folder(Path::new(FOLDER));
        Library {
            trash: MemoryTrash::new(files.clone(), "/trash"),
            files,
            hasher: FnvHasher,
            xml: QuickXml::new(),
            clock: ManualClock::new(
                Timestamp::from_unix_seconds(1_794_142_992),
                UtcOffset::from_seconds(0).unwrap(),
            ),
        }
    }

    fn add(&self, name: &str, content: &[u8]) {
        self.files
            .add_file(&path(name), content, Timestamp::UNIX_EPOCH);
    }

    /// Adds `<base>.RAF`, `<base>.JPG` and their own sidecars.
    fn add_pair_with_sidecars(&self, base_name: &str) {
        for name in ["RAF", "JPG"].map(|extension| format!("{base_name}.{extension}")) {
            self.add(&name, b"media");
            self.add(&format!("{name}.xmp"), EMPTY_SIDECAR.as_bytes());
        }
    }

    fn exists(&self, name: &str) -> bool {
        self.files.content(&path(name)).is_some()
    }

    fn sidecars(&self) -> SidecarFiles<'_> {
        SidecarFiles {
            files: &self.files,
            hasher: &self.hasher,
            xml: &self.xml,
            clock: &self.clock,
        }
    }

    fn keep_raw(&self, names: &[&str]) -> Ran {
        let files: Vec<PathBuf> = names.iter().map(|name| path(name)).collect();
        let (mut output, mut errors) = (Vec::new(), Vec::new());
        let exit_code = keep_raw(
            self.sidecars(),
            SidecarNaming::OriginalWithExtension,
            &files,
            KeepRawChange {
                marked: true,
                dry_run: false,
            },
            &mut output,
            &mut errors,
        );
        ran(exit_code, &output, &errors)
    }

    fn remove_raws(&self, answer: &str) -> Ran {
        self.run_remove_raws(answer, false)
    }

    fn run_remove_raws(&self, answer: &str, yes: bool) -> Ran {
        self.run_remove_raws_from(&mut answer.as_bytes(), yes)
    }

    fn run_remove_raws_from(&self, input: &mut dyn BufRead, yes: bool) -> Ran {
        let (mut output, mut errors) = (Vec::new(), Vec::new());
        let exit_code = remove_raws(
            self.sidecars(),
            &self.trash,
            Path::new(FOLDER),
            false,
            yes,
            &mut Terminal {
                input,
                output: &mut output,
                errors: &mut errors,
            },
        );
        ran(exit_code, &output, &errors)
    }
}

fn ran(exit_code: u8, output: &[u8], errors: &[u8]) -> Ran {
    Ran {
        exit_code,
        output: String::from_utf8(output.to_vec()).unwrap(),
        errors: String::from_utf8(errors.to_vec()).unwrap(),
    }
}

fn path(name: &str) -> PathBuf {
    Path::new(FOLDER).join(name)
}

fn paths(names: &[&str]) -> Vec<PathBuf> {
    names.iter().map(|name| path(name)).collect()
}

/// spec: CLI-018.4
#[test]
fn a_refused_marker_stops_only_its_asset() {
    let library = Library::new();
    library.add("A.RAF", b"raw");
    library.add("A.JPG", b"jpeg");
    library.add("A.RAF.xmp", MARKED_ARRAY.as_bytes());
    library.add("B.RAF", b"raw");
    library.add("B.JPG", b"jpeg");

    let ran = library.keep_raw(&["A.RAF", "B.RAF"]);

    assert_eq!(ran.exit_code, 1);
    assert!(
        ran.errors
            .starts_with("error: cannot set Keep RAW of /photos/A: the XMP sidecar /photos/A.RAF.xmp cannot be changed: "),
        "{}",
        ran.errors
    );
    assert_eq!(ran.output, "Keep RAW on: /photos/B\n");
    assert!(!library.exists("A.JPG.xmp"));
}

#[test]
fn an_asset_named_twice_is_changed_once() {
    let library = Library::new();
    library.add("A.RAF", b"raw");
    library.add("A.JPG", b"jpeg");
    library.add("A.RAF.xmp", EMPTY_SIDECAR.as_bytes());

    let ran = library.keep_raw(&["A.JPG", "A.RAF", "A.RAF.xmp"]);

    assert_eq!(ran.exit_code, 0, "{}", ran.errors);
    assert_eq!(ran.output, "Keep RAW on: /photos/A\n");
}

/// spec: CLI-018.6
#[test]
fn confirming_moves_the_raw_and_its_own_sidecar() {
    let library = Library::new();
    library.add_pair_with_sidecars("A");

    let ran = library.remove_raws("Y\n");

    assert_eq!(ran.exit_code, 0, "{}", ran.errors);
    assert_eq!(library.trash.moved(), paths(&["A.RAF", "A.RAF.xmp"]));
    assert!(library.exists("A.JPG") && library.exists("A.JPG.xmp"));
    assert!(ran.output.contains("Move 2 files to the trash? [y/N] "));
    assert!(
        ran.output
            .ends_with("Moved to the trash: 1 asset, 2 files, 0.0 MB freed\n")
    );
}

/// spec: CLI-018.12
#[test]
fn yes_moves_without_asking() {
    let library = Library::new();
    library.add_pair_with_sidecars("A");

    let ran = library.run_remove_raws("", true);

    assert_eq!(ran.exit_code, 0, "{}", ran.errors);
    assert_eq!(library.trash.moved(), paths(&["A.RAF", "A.RAF.xmp"]));
    assert!(!ran.output.contains("[y/N]"));
}

/// The answer `y`, given after `change` ran on the folder, as while the question is open.
struct ChangeThenYes<'a> {
    change: Option<Box<dyn FnOnce() + 'a>>,
    answer: &'a [u8],
}

impl std::io::Read for ChangeThenYes<'_> {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        self.fill_buf()?;
        let length = buffer.len().min(self.answer.len());
        buffer[..length].copy_from_slice(&self.answer[..length]);
        self.consume(length);
        Ok(length)
    }
}

impl BufRead for ChangeThenYes<'_> {
    fn fill_buf(&mut self) -> std::io::Result<&[u8]> {
        if let Some(change) = self.change.take() {
            change();
        }
        Ok(self.answer)
    }

    fn consume(&mut self, amount: usize) {
        self.answer = &self.answer[amount..];
    }
}

/// spec: CLI-018.14
#[test]
fn an_asset_that_changed_since_the_summary_keeps_its_raw() {
    let library = Library::new();
    library.add_pair_with_sidecars("A");
    library.add_pair_with_sidecars("B");
    let mut input = ChangeThenYes {
        change: Some(Box::new(|| {
            library.files.remove_file(&path("A.JPG")).unwrap();
        })),
        answer: b"y\n",
    };

    let ran = library.run_remove_raws_from(&mut input, false);

    assert_eq!(ran.exit_code, 1);
    assert_eq!(
        ran.errors,
        "warning: /photos/A changed since the summary: its rendered file is gone; its RAW stays\n"
    );
    assert!(library.exists("A.RAF") && library.exists("A.RAF.xmp"));
    assert_eq!(library.trash.moved(), paths(&["B.RAF", "B.RAF.xmp"]));
    assert!(
        ran.output
            .ends_with("Moved to the trash: 1 asset, 2 files, 0.0 MB freed\n")
    );
}

/// spec: CLI-018.8
#[test]
fn a_file_that_cannot_be_moved_is_reported() {
    let library = Library::new();
    library.add_pair_with_sidecars("A");
    library.add_pair_with_sidecars("B");
    library.trash.refuse(&path("A.RAF"));

    let ran = library.remove_raws("yes\n");

    assert_eq!(ran.exit_code, 1);
    assert!(
        ran.errors
            .starts_with("error: cannot move /photos/A.RAF to the trash: "),
        "{}",
        ran.errors
    );
    assert_eq!(ran.errors.lines().count(), 1);
    assert!(library.exists("A.RAF") && library.exists("A.RAF.xmp"));
    assert_eq!(library.trash.moved(), paths(&["B.RAF", "B.RAF.xmp"]));
    assert!(
        ran.output
            .ends_with("Moved to the trash: 1 asset, 2 files, 0.0 MB freed\n")
    );
}

/// spec: CLI-018.10
#[test]
fn an_unreadable_sidecar_keeps_its_asset_out() {
    let library = Library::new();
    library.add_pair_with_sidecars("A");
    library.add("A.JPG.xmp", b"not XMP");
    library.add_pair_with_sidecars("B");

    let ran = library.remove_raws("n\n");

    assert!(
        ran.errors
            .starts_with("warning: the XMP sidecar /photos/A.JPG.xmp cannot be read: "),
        "{}",
        ran.errors
    );
    assert!(ran.errors.ends_with("; its asset is left out\n"));
    assert!(ran.output.contains("  RAW will be removed (1): B\n"));
}

#[test]
fn an_ambiguous_base_name_is_warned_about() {
    let library = Library::new();
    library.add("A.RAF", b"raw");
    library.add("A.CR2", b"raw");

    let ran = library.remove_raws("");

    assert_eq!(
        ran.errors,
        "warning: /photos/A.*: more than one RAW or rendered file; each is an asset of its own\n"
    );
}

#[cfg(not(target_os = "macos"))]
#[test]
fn the_home_trash_follows_xdg_data_home_and_else_the_home_folder() {
    use std::ffi::OsStr;

    let home = Some(OsStr::new("/home/ada"));
    assert_eq!(
        home_trash(Some(OsStr::new("/data")), home),
        Some(PathBuf::from("/data/Trash"))
    );
    assert_eq!(
        home_trash(Some(OsStr::new("data")), home),
        Some(PathBuf::from("/home/ada/.local/share/Trash"))
    );
    assert_eq!(
        home_trash(None, home),
        Some(PathBuf::from("/home/ada/.local/share/Trash"))
    );
    assert_eq!(home_trash(None, Some(OsStr::new("relative"))), None);
}

/// spec: CLI-018.14
#[test]
fn every_change_has_its_reason_text() {
    use library::ChangeReason;
    let texts = [
        ChangeReason::RawGone,
        ChangeReason::RenderedGone,
        ChangeReason::MarkedKeepRaw,
        ChangeReason::Rejected,
        ChangeReason::StateUnreadable,
    ]
    .map(change_text);

    assert_eq!(
        texts,
        [
            "its RAW is gone",
            "its rendered file is gone",
            "it is marked Keep RAW",
            "it is rejected",
            "its state cannot be read",
        ]
    );
}
