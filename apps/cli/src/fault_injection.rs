// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The fault-injection point of the binary (ADR 0048): a [`FileSystem`] that wraps the
//! one of the import, compiled only with the cargo feature `fault-injection`.

use std::ffi::{OsStr, OsString};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock, PoisonError};

use domain::Timestamp;
use files::{EntryKind, FileSystem, FolderEntry, ReadOnlyFileSystem, ReadSeek, WriteFile};

/// The environment variable that names the fault of a run.
const VARIABLE: &str = "FRAMEHIVE_FAULT";

/// The fault of this run, read once at start by [`read_fault_from_environment`].
static FAULT: OnceLock<Option<FileFault>> = OnceLock::new();

/// A fault the decorator causes (ADR 0048 §3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Fault {
    /// Writes half of the first write to `<target>.part`, then aborts the process.
    AbortWhileWriting,
    /// Flips one bit of the first byte read back from the storage device.
    FlipBitAfterSync,
    /// Blocks the read of the source during its copy, for good.
    HangWhileReading,
    /// Fails the read of the source during its copy and every later call below its
    /// folder.
    UnplugWhileReading,
}

/// The name of each fault in [`VARIABLE`].
const FAULTS: [(&str, Fault); 4] = [
    ("abort-while-writing", Fault::AbortWhileWriting),
    ("flip-bit-after-sync", Fault::FlipBitAfterSync),
    ("hang-while-reading", Fault::HangWhileReading),
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
        if is_part(path) && self.is_due(Fault::AbortWhileWriting, path.file_stem()) {
            return Ok(Box::new(AbortingWriter { inner: file }));
        }
        Ok(file)
    }

    fn rename_no_replace(&self, from: &Path, to: &Path) -> io::Result<()> {
        self.check_plugged(from)?;
        self.set_copying(from, false);
        self.inner.rename_no_replace(from, to)
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

/// Writes half of its first write, then ends the process as a kill does.
struct AbortingWriter {
    inner: Box<dyn WriteFile>,
}

impl Write for AbortingWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        // The process ends here, so a failed write has no one to report to.
        let _ = self.inner.write_all(&bytes[..bytes.len() / 2]);
        std::process::abort()
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

impl WriteFile for AbortingWriter {
    fn set_modification_time(&mut self, time: Timestamp) -> io::Result<()> {
        self.inner.set_modification_time(time)
    }

    fn sync(&mut self) -> io::Result<()> {
        self.inner.sync()
    }
}

/// Flips the lowest bit of the first byte it returns.
struct BitFlippingReader {
    inner: Box<dyn ReadSeek>,
    flipped: bool,
}

impl Read for BitFlippingReader {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let count = self.inner.read(buffer)?;
        if let Some(first) = buffer[..count].first_mut().filter(|_| !self.flipped) {
            *first ^= 1;
            self.flipped = true;
        }
        Ok(count)
    }
}

impl Seek for BitFlippingReader {
    fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
        self.inner.seek(position)
    }
}

/// Blocks its first read while a `.part` file is written and never returns from it, so
/// reading the metadata or the checksum before the copy is not affected.
struct HangingReader {
    inner: Box<dyn ReadSeek>,
    state: Arc<Mutex<State>>,
}

impl Read for HangingReader {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let mut state = lock(&self.state);
        if state.copying && !state.fired {
            state.fired = true;
            drop(state);
            hang_forever();
        }
        drop(state);
        self.inner.read(buffer)
    }
}

impl Seek for HangingReader {
    fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
        self.inner.seek(position)
    }
}

/// Blocks the calling thread until the process ends, as a device that stopped answering.
fn hang_forever() -> ! {
    loop {
        // A spurious wake-up must not end the hang.
        std::thread::park();
    }
}

/// Unplugs `folder` at its first read while a `.part` file is written, so reading the
/// metadata or the checksum before the copy is not affected.
struct UnpluggingReader {
    inner: Box<dyn ReadSeek>,
    folder: PathBuf,
    state: Arc<Mutex<State>>,
}

impl Read for UnpluggingReader {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let mut state = lock(&self.state);
        if state.unplugged.is_some() {
            return Err(device_gone());
        }
        if state.copying && !state.fired {
            state.fired = true;
            state.unplugged = Some(self.folder.clone());
            return Err(device_gone());
        }
        drop(state);
        self.inner.read(buffer)
    }
}

impl Seek for UnpluggingReader {
    fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
        self.inner.seek(position)
    }
}

#[cfg(test)]
mod tests {
    use adapter_fs::StdFileSystem;
    use files::testing::{FileSystemUnderTest, TempDir, check_file_system};

    use super::{FaultyFileSystem, parse_fault};

    /// Without a fault, the decorator conforms to the port as the file system it wraps.
    /// spec: IMPORT-004.14
    #[test]
    fn conforms_to_the_file_system_port_without_a_fault() {
        let folder = TempDir::new().unwrap();
        let failures = check_file_system(&FileSystemUnderTest {
            file_system: &FaultyFileSystem::new(StdFileSystem::new(), None),
            root: folder.path(),
            crash: None,
            make_symbolic_link: Some(&|link, target| std::os::unix::fs::symlink(target, link)),
        });
        assert!(failures.is_empty(), "{failures:#?}");
    }

    /// A fault without a file name is refused, so a typo never runs without the fault.
    /// spec: CLI-025.4
    #[test]
    fn refuses_a_fault_without_a_file_name() {
        assert_eq!(
            parse_fault("flip-bit-after-sync"),
            Err("FRAMEHIVE_FAULT: `flip-bit-after-sync` needs a file name".to_owned())
        );
    }
}
