// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The counter file, its lock and the replacing rename (ADR 0029, section 4).

use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use naming::{SequenceCounters, SequenceCountersError, StoredCounters};

use crate::format::{self, Broken};

/// The name of the counter file in the state folder.
pub const COUNTER_FILE_NAME: &str = "sequence-counters.txt";

const LOCK_FILE_NAME: &str = "sequence-counters.lock";
const PART_FILE_NAME: &str = "sequence-counters.txt.part";

/// The persisted sequence counters of this machine, in `sequence-counters.txt` of a
/// state folder (ADR 0029).
///
/// An update holds an exclusive lock on `sequence-counters.lock` in the same folder,
/// so imports in several processes get ranges one after the other. A broken file
/// stops every read and update and is never overwritten.
#[derive(Debug, Clone)]
pub struct FileSequenceCounters {
    folder: PathBuf,
}

impl FileSequenceCounters {
    /// Counters in `folder`, the state folder of ADR 0025, section 1. The folder is
    /// created by the first update.
    #[must_use]
    pub fn new(folder: PathBuf) -> Self {
        FileSequenceCounters { folder }
    }

    /// The counter file.
    #[must_use]
    pub fn file(&self) -> PathBuf {
        self.folder.join(COUNTER_FILE_NAME)
    }

    fn unavailable(path: &Path, error: &io::Error) -> SequenceCountersError {
        SequenceCountersError::Unavailable {
            file: path.to_path_buf(),
            message: error.to_string(),
        }
    }

    /// Opens the lock file and waits for the exclusive lock; closing the file
    /// releases it.
    fn lock(&self) -> Result<File, SequenceCountersError> {
        let path = self.folder.join(LOCK_FILE_NAME);
        let locked = fs::create_dir_all(&self.folder)
            .and_then(|()| {
                OpenOptions::new()
                    .create(true)
                    .truncate(false)
                    .write(true)
                    .open(&path)
            })
            .and_then(|file| file.lock().map(|()| file));
        locked.map_err(|error| Self::unavailable(&path, &error))
    }

    /// Writes `counters` to the part file, syncs it, renames it over the counter file
    /// and syncs the folder, so a crash leaves the old or the new file.
    fn store(&self, counters: &StoredCounters) -> Result<(), SequenceCountersError> {
        let part = self.folder.join(PART_FILE_NAME);
        write_synced(&part, &format::format(counters))
            .map_err(|error| Self::unavailable(&part, &error))?;
        let file = self.file();
        fs::rename(&part, &file).map_err(|error| Self::unavailable(&file, &error))?;
        File::open(&self.folder)
            .and_then(|folder| folder.sync_all())
            .map_err(|error| Self::unavailable(&self.folder, &error))
    }
}

impl SequenceCounters for FileSequenceCounters {
    fn read(&self) -> Result<StoredCounters, SequenceCountersError> {
        // The rename replaces the file in one step, so reading needs no lock.
        let file = self.file();
        let text = match fs::read_to_string(&file) {
            Ok(text) => text,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return Ok(StoredCounters::default());
            }
            Err(error) => return Err(Self::unavailable(&file, &error)),
        };
        format::parse(&text).map_err(|Broken { line, reason }| SequenceCountersError::Broken {
            file,
            line,
            reason,
        })
    }

    fn update(
        &self,
        change: &mut dyn FnMut(StoredCounters) -> Option<StoredCounters>,
    ) -> Result<StoredCounters, SequenceCountersError> {
        let _lock = self.lock()?;
        let before = self.read()?;
        if let Some(after) = change(before) {
            self.store(&after)?;
        }
        Ok(before)
    }
}

fn write_synced(path: &Path, text: &str) -> io::Result<()> {
    let mut file = File::create(path)?;
    file.write_all(text.as_bytes())?;
    file.sync_all()
}
