// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of `StallGuard`, the stall limit of a source (ADR 0049).

#![allow(clippy::unwrap_used, reason = "test helpers may panic on bad fixtures")]
#![allow(clippy::panic, reason = "a test fails by panicking")]

use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::{self, Sender};
use std::thread;
use std::time::Duration;

use domain::Timestamp;
use files::testing::{
    Fault, Hang, ManualTimer, MemoryFileSystem, ReadOnlyFileSystemUnderTest, SlowReads,
    check_read_only_file_system,
};
use files::{
    EntryKind, FolderEntry, ReadOnlyFileSystem, ReadSeek, SourceStall, StallGuard, source_stall,
};

const LIMIT: Duration = Duration::from_secs(30);
const PHOTO: &str = "/DCIM/100_FUJI/DSCF0001.RAF";
const FOLDER: &str = "/DCIM/100_FUJI";

fn source_with_photo(content: &[u8]) -> MemoryFileSystem {
    let source = MemoryFileSystem::new();
    source.add_file(Path::new(PHOTO), content, Timestamp::UNIX_EPOCH);
    source
}

fn guard(source: &MemoryFileSystem, timer: &Arc<ManualTimer>) -> StallGuard {
    StallGuard::new(Box::new(source.clone()), Arc::clone(timer) as _, LIMIT).unwrap()
}

/// Runs `call` on a thread of its own and returns once it hangs and waits for the
/// timer.
fn hang_in_thread<T: Send + 'static>(
    hang: &Hang,
    timer: &ManualTimer,
    call: impl FnOnce() -> io::Result<T> + Send + 'static,
) -> thread::JoinHandle<io::Result<T>> {
    let handle = thread::spawn(call);
    hang.wait_until_begun();
    while timer.waits() == 0 {
        thread::yield_now();
    }
    handle
}

fn read_the_photo(guard: &Arc<StallGuard>) -> impl FnOnce() -> io::Result<usize> + Send + use<> {
    let guard = Arc::clone(guard);
    move || guard.open_read(Path::new(PHOTO))?.read(&mut [0; 8])
}

fn stall_of<T>(result: io::Result<T>) -> (io::ErrorKind, String, Option<SourceStall>) {
    let Err(error) = result else {
        panic!("the call answered");
    };
    (
        error.kind(),
        error.to_string(),
        source_stall(&error).copied(),
    )
}

#[test]
fn conforms_to_the_read_only_file_system_port() {
    let source = MemoryFileSystem::new();
    source.add_folder(Path::new("/root"));
    let guard = guard(&source, &Arc::new(ManualTimer::new()));
    let add_folder = |path: &Path| {
        source.add_folder(path);
        Ok(())
    };
    let add_file = |path: &Path, content: &[u8], modified: Timestamp| {
        source.add_file(path, content, modified);
        Ok(())
    };

    let failures = check_read_only_file_system(&ReadOnlyFileSystemUnderTest {
        file_system: &guard,
        root: Path::new("/root"),
        add_folder: &add_folder,
        add_file: &add_file,
        make_symbolic_link: None,
    });

    assert_eq!(failures, Vec::<String>::new());
}

#[test]
fn errors_of_the_source_are_not_stalls() {
    let guard = guard(&MemoryFileSystem::new(), &Arc::new(ManualTimer::new()));

    let error = guard.file_size(Path::new("/missing.RAF")).unwrap_err();

    assert_eq!(error.kind(), io::ErrorKind::NotFound);
    assert_eq!(source_stall(&error), None);
    assert!(!guard.is_stopped());
}

#[test]
fn a_hanging_read_fails_at_the_limit_and_stops_the_source() {
    let (source, hang) = (source_with_photo(b"raw"), Hang::new());
    source.inject_fault(Path::new(PHOTO), Fault::Hangs(hang.clone()));
    let timer = Arc::new(ManualTimer::new());
    let guard = Arc::new(guard(&source, &timer));
    let reading = hang_in_thread(&hang, &timer, read_the_photo(&guard));

    timer.advance(Duration::from_secs(29));
    assert_eq!(timer.waits(), 1, "the read failed before the limit");
    timer.advance(Duration::from_secs(1));

    let stall = stall_of(reading.join().unwrap());
    assert_eq!(
        stall,
        (
            io::ErrorKind::TimedOut,
            "the source did not answer for 30 seconds".to_owned(),
            Some(SourceStall::DidNotAnswer { limit_seconds: 30 })
        )
    );
    assert!(guard.is_stopped());
    hang.release();
}

#[test]
fn a_hanging_listing_fails_at_the_limit() {
    let (source, hang) = (source_with_photo(b"raw"), Hang::new());
    source.inject_fault(Path::new(FOLDER), Fault::Hangs(hang.clone()));
    let timer = Arc::new(ManualTimer::new());
    let guard = Arc::new(guard(&source, &timer));
    let listing = {
        let guard = Arc::clone(&guard);
        hang_in_thread(&hang, &timer, move || guard.list_folder(Path::new(FOLDER)))
    };

    timer.advance(LIMIT);

    let (_, message, stall) = stall_of(listing.join().unwrap());
    assert_eq!(message, "the source did not answer for 30 seconds");
    assert_eq!(stall, Some(SourceStall::DidNotAnswer { limit_seconds: 30 }));
    hang.release();
}

/// The source thread is still blocked in the hanging read, so a call that reached it
/// would wait for the timer, which no longer advances.
#[test]
fn a_stopped_source_fails_every_call_at_once_without_making_it() {
    let (source, hang) = (source_with_photo(b"raw"), Hang::new());
    source.inject_fault(Path::new(PHOTO), Fault::Hangs(hang.clone()));
    let timer = Arc::new(ManualTimer::new());
    let guard = Arc::new(guard(&source, &timer));
    let reading = hang_in_thread(&hang, &timer, read_the_photo(&guard));
    timer.advance(LIMIT);
    stall_of(reading.join().unwrap());

    let calls = [
        stall_of(guard.entry(Path::new(PHOTO))),
        stall_of(guard.list_folder(Path::new(FOLDER))),
        stall_of(guard.file_size(Path::new(PHOTO))),
        stall_of(guard.open_read(Path::new(PHOTO))),
        stall_of(guard.modification_time(Path::new(PHOTO))),
    ];

    let stopped = (
        io::ErrorKind::Other,
        "the source stopped answering".to_owned(),
        Some(SourceStall::StoppedAnswering),
    );
    assert_eq!(calls, [(); 5].map(|()| stopped.clone()));
    assert_eq!(timer.waits(), 0);
    hang.release();
}

#[test]
fn a_slow_source_that_keeps_answering_is_not_stopped() {
    let content: Vec<u8> = (0..=u8::MAX).collect();
    let source = source_with_photo(&content);
    let timer = Arc::new(ManualTimer::new());
    let each_read = Duration::from_secs(20);
    let slow = SlowReads::new(Arc::clone(&timer), each_read);
    source.inject_fault(Path::new(PHOTO), Fault::SlowReads(slow));
    let guard = guard(&source, &timer);
    let mut file = guard.open_read(Path::new(PHOTO)).unwrap();

    let mut read = Vec::new();
    let mut chunk = [0; 64];
    while let count @ 1.. = file.read(&mut chunk).unwrap() {
        read.extend_from_slice(&chunk[..count]);
    }

    assert_eq!(read, content);
    assert_eq!(timer.time_since_start(), each_read * 5);
    assert!(!guard.is_stopped());
}

/// A source whose drop is seen through a channel that closes with it.
struct DropSignalling {
    _dropped: Sender<()>,
}

impl ReadOnlyFileSystem for DropSignalling {
    fn entry(&self, _: &Path) -> io::Result<Option<EntryKind>> {
        Ok(None)
    }

    fn list_folder(&self, _: &Path) -> io::Result<Vec<FolderEntry>> {
        Ok(Vec::new())
    }

    fn file_size(&self, _: &Path) -> io::Result<u64> {
        Ok(0)
    }

    fn open_read(&self, _: &Path) -> io::Result<Box<dyn ReadSeek>> {
        Ok(Box::new(io::Cursor::new(Vec::new())))
    }

    fn modification_time(&self, _: &Path) -> io::Result<Timestamp> {
        Ok(Timestamp::UNIX_EPOCH)
    }
}

#[test]
fn dropping_the_guard_and_its_files_drops_the_source() {
    let (dropped, source_dropped) = mpsc::channel();
    let source = Box::new(DropSignalling { _dropped: dropped });
    let guard = StallGuard::new(source, Arc::new(ManualTimer::new()), LIMIT).unwrap();
    let file = guard.open_read(&PathBuf::from(PHOTO)).unwrap();

    drop(guard);
    drop(file);

    assert_eq!(source_dropped.recv(), Err(mpsc::RecvError));
}
