// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Sidecar files: their names, finding them, and merging into them through the
//! `files` port (CORE-009, sidecar names and saving).

#![allow(clippy::unwrap_used, reason = "tests may panic on bad fixtures")]

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};

use domain::testing::ManualClock;
use domain::{Timestamp, UtcOffset};
use files::testing::{Fault, FnvHasher, MemoryFileSystem};
use metadata::xmp::namespaces::{DUBLIN_CORE, PHOTOSHOP, XMP_BASIC};
use metadata::xmp::{
    SidecarError, SidecarErrorKind, SidecarFiles, SidecarNaming, SidecarStep, XmlEvent, XmlSyntax,
    XmlSyntaxError, XmlSyntaxErrorKind, XmpErrorReason, XmpName, XmpNode, XmpPacket, find_sidecars,
    serialize_xmp,
};

const SIDECAR: &str = "/photos/DSCF0042.RAF.xmp";
const PART: &str = "/photos/DSCF0042.RAF.xmp.part";

/// `2026-11-08T14:03:12+01:00`, the time of every write.
const WRITE_TIME: &str = "2026-11-08T14:03:12+01:00";

/// An [`XmlSyntax`] that reads back only the text it wrote; any other text is
/// malformed. It keeps these tests free of an XML adapter, which a service may not
/// depend on (architecture §3); `adapter-xmp` tests the real XML.
#[derive(Default)]
struct RecordedXml(RefCell<BTreeMap<String, Vec<XmlEvent>>>);

impl XmlSyntax for RecordedXml {
    fn read(&self, text: &str) -> Result<Vec<XmlEvent>, XmlSyntaxError> {
        self.0
            .borrow()
            .get(text.trim_end())
            .cloned()
            .ok_or_else(|| XmlSyntaxError {
                offset: 0,
                kind: XmlSyntaxErrorKind::Malformed("not written by this test".to_owned()),
            })
    }

    fn write(&self, events: &[XmlEvent]) -> String {
        let text = format!("<recorded {}/>", self.0.borrow().len());
        self.0.borrow_mut().insert(text.clone(), events.to_vec());
        text
    }
}

/// The ports of [`SidecarFiles`], with the files in memory.
struct Sidecars {
    files: MemoryFileSystem,
    hasher: FnvHasher,
    xml: RecordedXml,
    clock: ManualClock,
}

impl Sidecars {
    /// A folder `/photos`, and the sidecar there with `content` if given.
    fn new(content: Option<&[u8]>) -> Self {
        let files = MemoryFileSystem::new();
        files.add_folder(Path::new("/photos"));
        if let Some(content) = content {
            files.add_file(Path::new(SIDECAR), content, Timestamp::UNIX_EPOCH);
        }
        Sidecars {
            files,
            hasher: FnvHasher,
            xml: RecordedXml::default(),
            clock: ManualClock::new(
                Timestamp::from_unix_seconds(1_794_142_992),
                UtcOffset::from_seconds(3_600).unwrap(),
            ),
        }
    }

    /// A folder `/photos` with a sidecar whose `dc:subject` is `Beach`, `Sea`.
    fn with_subject() -> Self {
        let sidecars = Sidecars::new(None);
        let mut packet = XmpPacket::new();
        packet
            .add_to_bag(&dc("subject"), &["Beach", "Sea"])
            .unwrap();
        let content = serialize_xmp(&packet, &sidecars.clock, &sidecars.xml).unwrap();
        sidecars.files.add_file(
            Path::new(SIDECAR),
            content.as_bytes(),
            Timestamp::UNIX_EPOCH,
        );
        sidecars
    }

    /// Parses the sidecar.
    fn read(&self) -> XmpPacket {
        let ports = self.ports();
        ports.read(Path::new(SIDECAR)).unwrap().packet
    }

    fn ports(&self) -> SidecarFiles<'_> {
        SidecarFiles {
            files: &self.files,
            hasher: &self.hasher,
            xml: &self.xml,
            clock: &self.clock,
        }
    }

    /// Sets the job code `Lisbon` in the sidecar.
    fn merge_job_code(&self) -> Result<(), SidecarError> {
        self.ports().merge(Path::new(SIDECAR), |packet| {
            packet.set_text(&photoshop("TransmissionReference"), "Lisbon")
        })
    }

    fn sidecar(&self) -> Option<Vec<u8>> {
        self.files.content(Path::new(SIDECAR))
    }

    fn part_is_gone(&self) -> bool {
        self.files.content(Path::new(PART)).is_none()
    }
}

fn the_photo_sidecar(error: &SidecarError) -> bool {
    error.path == Path::new(SIDECAR) && error.part_left.is_none()
}

#[test]
fn a_merge_keeps_the_other_values() {
    let sidecars = Sidecars::with_subject();
    let subject = sidecars.read().property(&dc("subject")).cloned();

    sidecars.merge_job_code().unwrap();

    let written = sidecars.read();
    assert_eq!(written.property(&dc("subject")).cloned(), subject);
    let job_code = written.property(&photoshop("TransmissionReference"));
    assert_eq!(job_code, Some(&XmpNode::text("Lisbon")));
    let metadata_date = written.property(&XmpName::new(XMP_BASIC, "MetadataDate"));
    assert_eq!(metadata_date, Some(&XmpNode::text(WRITE_TIME)));
    assert!(sidecars.part_is_gone());
}

#[test]
fn a_merge_without_a_sidecar_creates_one() {
    let sidecars = Sidecars::new(None);

    sidecars.merge_job_code().unwrap();

    let job_code = sidecars
        .read()
        .property(&photoshop("TransmissionReference"))
        .cloned();
    assert_eq!(job_code, Some(XmpNode::text("Lisbon")));
    assert!(sidecars.part_is_gone());
}

/// spec: CORE-009.15
#[test]
fn a_sidecar_that_cannot_be_parsed_is_kept() {
    let unclosed = b"<x:xmpmeta><rdf:RDF>".to_vec();
    let sidecars = Sidecars::new(Some(&unclosed));

    let error = sidecars.merge_job_code().unwrap_err();

    assert!(
        matches!(
            error.kind,
            SidecarErrorKind::Unreadable(XmpErrorReason::Malformed { .. })
        ),
        "{error}"
    );
    assert!(the_photo_sidecar(&error), "{error:?}");
    assert_eq!(sidecars.sidecar(), Some(unclosed));
    assert!(sidecars.part_is_gone());
}

/// spec: CORE-009.16
#[test]
fn a_failed_rename_leaves_the_sidecar_as_before() {
    for sidecars in [Sidecars::with_subject(), Sidecars::new(None)] {
        let before = sidecars.sidecar();
        sidecars
            .files
            .inject_fault(Path::new(SIDECAR), Fault::RenameFails);

        let error = sidecars.merge_job_code().unwrap_err();

        assert!(
            matches!(
                error.kind,
                SidecarErrorKind::Io {
                    step: SidecarStep::Rename,
                    ..
                }
            ),
            "{error}"
        );
        assert!(the_photo_sidecar(&error), "{error:?}");
        assert!(error.to_string().contains(SIDECAR), "{error}");
        assert_eq!(sidecars.sidecar(), before);
        assert!(sidecars.part_is_gone());
    }
}

/// spec: CORE-009.17
#[test]
fn a_sidecar_that_appeared_is_not_replaced() {
    let sidecars = Sidecars::new(None);
    other_program_writes_the_sidecar(&sidecars, b"theirs");

    let error = sidecars.merge_job_code().unwrap_err();

    assert!(matches!(error.kind, SidecarErrorKind::Appeared), "{error}");
    assert!(the_photo_sidecar(&error), "{error:?}");
    assert_eq!(sidecars.sidecar(), Some(b"theirs".to_vec()));
    assert!(sidecars.part_is_gone());
}

/// spec: CORE-009.18
#[test]
fn a_sidecar_changed_meanwhile_is_not_replaced() {
    let sidecars = Sidecars::with_subject();
    // Same size and modification time, so only the checksum tells the change.
    let mut changed = sidecars.sidecar().unwrap();
    let last = changed.len() - 2;
    changed[last] ^= 1;
    other_program_writes_the_sidecar(&sidecars, &changed);

    let error = sidecars.merge_job_code().unwrap_err();

    assert!(matches!(error.kind, SidecarErrorKind::Changed), "{error}");
    assert!(the_photo_sidecar(&error), "{error:?}");
    assert_eq!(sidecars.sidecar(), Some(changed));
    assert!(sidecars.part_is_gone());
}

/// spec: CORE-009.19
#[test]
fn a_read_only_sidecar_is_kept() {
    let sidecars = Sidecars::with_subject();
    let before = sidecars.sidecar();
    sidecars.files.set_read_only(Path::new(SIDECAR));

    let error = sidecars.merge_job_code().unwrap_err();

    let SidecarErrorKind::Io { step, source } = &error.kind else {
        panic!("{error}");
    };
    assert_eq!(*step, SidecarStep::Rename);
    assert_eq!(source.kind(), io::ErrorKind::PermissionDenied);
    assert!(the_photo_sidecar(&error), "{error:?}");
    assert_eq!(sidecars.sidecar(), before);
    assert!(sidecars.part_is_gone());
}

/// spec: CORE-009.20
#[test]
fn both_sidecar_names_are_found() {
    let files = MemoryFileSystem::new();
    for name in [
        "DSCF0042.RAF",
        "DSCF0042.RAF.XMP",
        "DSCF0043.RAF",
        "DSCF0043.xmp",
        "DSCF0044.RAF",
    ] {
        files.add_file(&photos(name), b"", Timestamp::UNIX_EPOCH);
    }

    let found = |name| find_sidecars(&files, &photos(name)).unwrap();

    assert_eq!(found("DSCF0042.RAF"), [photos("DSCF0042.RAF.XMP")]);
    assert_eq!(found("DSCF0043.RAF"), [photos("DSCF0043.xmp")]);
    assert_eq!(found("DSCF0044.RAF"), Vec::<PathBuf>::new());
}

#[test]
fn the_sidecar_named_after_the_full_name_comes_first() {
    let files = MemoryFileSystem::new();
    for name in [
        "DSCF0042.RAF",
        "DSCF0042.xmp",
        "DSCF0042.RAF.xmp",
        "DSCF0042.JPG.xmp",
    ] {
        files.add_file(&photos(name), b"", Timestamp::UNIX_EPOCH);
    }
    files.add_folder(&photos("DSCF0042.XMP"));

    let found = find_sidecars(&files, &photos("DSCF0042.RAF")).unwrap();

    assert_eq!(found, [photos("DSCF0042.RAF.xmp"), photos("DSCF0042.xmp")]);
}

/// spec: CORE-009.21
#[test]
fn names_follow_the_naming_setting() {
    let file = photos("20261108-0042.RAF");

    let with = SidecarNaming::OriginalWithExtension.sidecar_path(&file);
    let without = SidecarNaming::OriginalWithoutExtension.sidecar_path(&file);

    assert_eq!(with, photos("20261108-0042.RAF.xmp"));
    assert_eq!(without, photos("20261108-0042.xmp"));
    assert_eq!(
        SidecarNaming::default(),
        SidecarNaming::OriginalWithExtension
    );
}

/// Lets another program write `content` to the sidecar when the `.part` file is
/// written, before the rename.
fn other_program_writes_the_sidecar(sidecars: &Sidecars, content: &[u8]) {
    sidecars.files.inject_fault(
        Path::new(PART),
        Fault::OtherProgramCreates {
            path: PathBuf::from(SIDECAR),
            content: content.to_vec(),
        },
    );
}

fn photos(name: &str) -> PathBuf {
    Path::new("/photos").join(name)
}

fn dc(local: &str) -> XmpName {
    XmpName::new(DUBLIN_CORE, local)
}

fn photoshop(local: &str) -> XmpName {
    XmpName::new(PHOTOSHOP, local)
}
