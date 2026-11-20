// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The fault-injection point of the binary (ADR 0048): a [`FileSystem`] that wraps the
//! one of the import, compiled only with the cargo feature `fault-injection`.

use std::ffi::{OsStr, OsString};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock, PoisonError};

use domain::Timestamp;
use files::{EntryKind, FileSystem, FolderEntry, ReadOnlyFileSystem, ReadSeek, WriteFile};

/// The environment variable that names the fault of a run.
const VARIABLE: &str = "FRAMEHIVE_FAULT";

/// The fault of this run, read once at start by [`read_fault_from_environment`].
static FAULT: OnceLock<Option<FileFault>> = OnceLock::new();

/// The environment variable of the mount table the guided import reads (ADR 0050).
const MOUNT_TABLE_VARIABLE: &str = "FRAMEHIVE_MOUNTINFO";

/// Set when the card was pulled, so the guided import polls the devices at once.
static POLL_FORCED: AtomicBool = AtomicBool::new(false);

/// A fault the decorator causes (ADR 0048 §3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Fault {
    /// Writes half of the first write to `<target>.part`, then aborts the process.
    AbortWhileWriting,
    /// Flips one bit of the first byte read back from the storage device.
    FlipBitAfterSync,
    /// Blocks the read of the source during its copy, for good.
    HangWhileReading,
    /// Panics when the `.part` file of the target is created, in the import job.
    PanicWhileWriting,
    /// Empties the mount table once the target is complete, and makes the guided import
    /// poll the devices at once.
    PullCardAfter,
    /// Fails the read of the source during its copy and every later call below its
    /// folder.
    UnplugWhileReading,
}

/// The name of each fault in [`VARIABLE`].
const FAULTS: [(&str, Fault); 6] = [
    ("abort-while-writing", Fault::AbortWhileWriting),
    ("flip-bit-after-sync", Fault::FlipBitAfterSync),
    ("hang-while-reading", Fault::HangWhileReading),
    ("panic-while-writing", Fault::PanicWhileWriting),
    ("pull-card-after", Fault::PullCardAfter),
    ("unplug-while-reading", Fault::UnplugWhileReading),
];

/// A fault and the name of the file it applies to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FileFault {
    fault: Fault,
    file_name: OsString,
}

/// Reads [`VARIABLE`] and keeps its fault for [`configured_fault`].
///
/// # Errors
///
/// Returns the message of a usage error when the value names no known fault or no file.
pub(crate) fn read_fault_from_environment() -> Result<(), String> {
    let fault = match std::env::var(VARIABLE) {
        Ok(value) => Some(parse_fault(&value)?),
        Err(std::env::VarError::NotPresent) => None,
        Err(std::env::VarError::NotUnicode(_)) => {
            return Err(format!("{VARIABLE} is not valid UTF-8"));
        }
    };
    // Set once at start; a second call keeps the first value.
    let _ = FAULT.set(fault);
    Ok(())
}

/// The fault read at start, if any.
pub(crate) fn configured_fault() -> Option<FileFault> {
    FAULT.get().cloned().flatten()
}

/// Parses `<fault> <file name>`.
fn parse_fault(value: &str) -> Result<FileFault, String> {
    let value = value.trim();
    let (name, file_name) = value.split_once(' ').unwrap_or((value, ""));
    let fault = FAULTS
        .iter()
        .find(|(known, _)| *known == name)
        .map(|(_, fault)| *fault)
        .ok_or_else(|| {
            let known = FAULTS.map(|(known, _)| known).join(", ");
            format!("{VARIABLE}: unknown fault `{name}`; the faults are {known}")
        })?;
    let file_name = file_name.trim();
    if file_name.is_empty() {
        return Err(format!("{VARIABLE}: `{name}` needs a file name"));
    }
    Ok(FileFault {
        fault,
        file_name: OsString::from(file_name),
    })
}

/// Whether the card was pulled since the last call; the guided import then polls the
/// devices without waiting for its interval.
pub(crate) fn take_forced_poll() -> bool {
    POLL_FORCED.swap(false, Ordering::SeqCst)
}

/// Replaces the mount table whole with an empty one, as a poll must not see it half
/// written, and forces the next poll. Without a mount table nothing is pulled.
fn pull_card() {
    let Some(table) = std::env::var_os(MOUNT_TABLE_VARIABLE).map(PathBuf::from) else {
        return;
    };
    let next = table.with_extension("pulled");
    // A fault that cannot be set up must not look like a pulled card.
    if std::fs::write(&next, "").is_ok() && std::fs::rename(&next, &table).is_ok() {
        POLL_FORCED.store(true, Ordering::SeqCst);
    }
}

/// What the decorator has seen of the run so far.
#[derive(Default)]
struct State {
    /// The fault has happened; a run causes it once.
    fired: bool,
    /// A `.part` file is being written, so a read of the source now is its copy.
    copying: bool,
    /// The folder of the unplugged source.
    unplugged: Option<PathBuf>,
}

/// A [`FileSystem`] that passes every call through to the one it wraps, except for the
/// file its fault names.
pub(crate) struct FaultyFileSystem<F> {
    inner: F,
    fault: Option<FileFault>,
    state: Arc<Mutex<State>>,
}

impl<F: FileSystem> FaultyFileSystem<F> {
    /// Wraps `inner`, causing `fault` if there is one.
    pub(crate) fn new(inner: F, fault: Option<FileFault>) -> Self {
        FaultyFileSystem {
            inner,
            fault,
            state: Arc::default(),
        }
    }

    /// Whether `fault` is set, has not happened yet and names the file `name`.
    fn is_due(&self, fault: Fault, name: Option<&OsStr>) -> bool {
        let named = self
            .fault
            .as_ref()
            .is_some_and(|set| set.fault == fault && name == Some(set.file_name.as_os_str()));
        named && !lock(&self.state).fired
    }

    /// Fails a call on `path` once its folder is unplugged.
    fn check_plugged(&self, path: &Path) -> io::Result<()> {
        match &lock(&self.state).unplugged {
            Some(folder) if path.starts_with(folder) => Err(device_gone()),
            _ => Ok(()),
        }
    }

    fn set_copying(&self, path: &Path, copying: bool) {
        if is_part(path) {
            lock(&self.state).copying = copying;
        }
    }
}

fn lock(state: &Mutex<State>) -> MutexGuard<'_, State> {
    state.lock().unwrap_or_else(PoisonError::into_inner)
}

/// The error of a device that is gone: `EIO`, as the kernel reports it.
fn device_gone() -> io::Error {
    io::Error::from_raw_os_error(5)
}

fn is_part(path: &Path) -> bool {
    path.extension() == Some(OsStr::new("part"))
}

impl<F: FileSystem> ReadOnlyFileSystem for FaultyFileSystem<F> {
    fn entry(&self, path: &Path) -> io::Result<Option<EntryKind>> {
        self.check_plugged(path)?;
        self.inner.entry(path)
    }

    fn list_folder(&self, path: &Path) -> io::Result<Vec<FolderEntry>> {
        self.check_plugged(path)?;
        self.inner.list_folder(path)
    }

    fn file_size(&self, path: &Path) -> io::Result<u64> {
        self.check_plugged(path)?;
        self.inner.file_size(path)
    }

    fn open_read(&self, path: &Path) -> io::Result<Box<dyn ReadSeek>> {
        self.check_plugged(path)?;
        let reader = self.inner.open_read(path)?;
        if self.is_due(Fault::HangWhileReading, path.file_name()) {
            return Ok(Box::new(HangingReader {
                inner: reader,
                state: Arc::clone(&self.state),
            }));
        }
        if !self.is_due(Fault::UnplugWhileReading, path.file_name()) {
            return Ok(reader);
        }
        Ok(Box::new(UnpluggingReader {
            inner: reader,
            folder: path.parent().unwrap_or(path).to_path_buf(),
            state: Arc::clone(&self.state),
        }))
    }

    fn modification_time(&self, path: &Path) -> io::Result<Timestamp> {
        self.check_plugged(path)?;
        self.inner.modification_time(path)
    }
}

impl<F: FileSystem> FileSystem for FaultyFileSystem<F> {
    fn create_folder(&self, path: &Path) -> io::Result<()> {
        self.check_plugged(path)?;
        self.inner.create_folder(path)
    }

    fn create_file(&self, path: &Path) -> io::Result<Box<dyn WriteFile>> {
        self.check_plugged(path)?;
        let file = self.inner.create_file(path)?;
        self.set_copying(path, true);
        if is_part(path) && self.is_due(Fault::PanicWhileWriting, path.file_stem()) {
            lock(&self.state).fired = true;
            #[allow(clippy::panic, reason = "the fault is a panic in the import job")]
            {
                panic!("{VARIABLE}: panic-while-writing");
            }
        }
        if is_part(path) && self.is_due(Fault::AbortWhileWriting, path.file_stem()) {
            return Ok(Box::new(AbortingWriter { inner: file }));
        }
        Ok(file)
    }

    fn rename_no_replace(&self, from: &Path, to: &Path) -> io::Result<()> {
        self.check_plugged(from)?;
        self.set_copying(from, false);
        self.inner.rename_no_replace(from, to)?;
        if self.is_due(Fault::PullCardAfter, to.file_name()) {
            lock(&self.state).fired = true;
            pull_card();
        }
        Ok(())
    }

    fn replace_file(&self, from: &Path, to: &Path) -> io::Result<()> {
        self.check_plugged(from)?;
        self.inner.replace_file(from, to)
    }

    fn remove_file(&self, path: &Path) -> io::Result<()> {
        self.check_plugged(path)?;
        self.set_copying(path, false);
        self.inner.remove_file(path)
    }

    fn sync_folder(&self, path: &Path) -> io::Result<()> {
        self.check_plugged(path)?;
        self.inner.sync_folder(path)
    }

    fn open_read_from_medium(&self, path: &Path) -> io::Result<Box<dyn ReadSeek>> {
        self.check_plugged(path)?;
        let reader = self.inner.open_read_from_medium(path)?;
        if !self.is_due(Fault::FlipBitAfterSync, path.file_name()) {
            return Ok(reader);
        }
        lock(&self.state).fired = true;
        Ok(Box::new(BitFlippingReader {
            inner: reader,
            flipped: false,
        }))
    }
}

mod doubles;
use doubles::{AbortingWriter, BitFlippingReader, HangingReader, UnpluggingReader};

#[cfg(test)]
mod tests;
