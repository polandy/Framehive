// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The `freedesktop-trash` adapter of Framehive: it implements the [`Trash`] port of
//! `files` after the freedesktop.org Trash specification, in own code (ADR 0043). Only
//! apps depend on it. See `README.md`.

#![forbid(unsafe_code)]

mod trash_folder;
mod trash_info;

use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

use domain::{Clock, LocalDateTime};
use files::{Trash, not_a_file};

use trash_folder::TrashFolder;

/// The trash of the freedesktop.org Trash specification 1.0, as GNOME, KDE and other
/// Linux desktops use it (ADR 0043).
///
/// A file on the file system of the home trash goes there; a file on another file
/// system goes to the trash at the top folder of that file system, `.Trash/<uid>` or
/// `.Trash-<uid>`. Before the move, the trash writes the `.trashinfo` file that names
/// the original path and the deletion time in local time, so the desktop can restore it.
///
/// ```
/// use files::Trash;
/// use adapter_freedesktop_trash::FreedesktopTrash;
/// use domain::testing::ManualClock;
/// use domain::{Timestamp, UtcOffset};
///
/// let folder = files::testing::TempDir::new()?;
/// let clock = ManualClock::new(Timestamp::from_unix_seconds(0), UtcOffset::UTC);
/// let trash = FreedesktopTrash::new(folder.path().join("Trash"), clock);
/// let photo = folder.path().join("DSCF0001.RAF");
/// std::fs::write(&photo, b"raw")?;
///
/// let in_trash = trash.move_to_trash(&photo)?;
/// assert_eq!(in_trash, folder.path().join("Trash/files/DSCF0001.RAF"));
/// # Ok::<(), std::io::Error>(())
/// ```
#[derive(Debug, Clone)]
pub struct FreedesktopTrash<C> {
    home_trash: PathBuf,
    clock: C,
}

impl<C: Clock> FreedesktopTrash<C> {
    /// Creates the trash with the home trash at `home_trash`, which is
    /// `$XDG_DATA_HOME/Trash`; it is created when the first file goes there. `clock`
    /// gives the deletion time.
    #[must_use]
    pub fn new(home_trash: PathBuf, clock: C) -> Self {
        FreedesktopTrash { home_trash, clock }
    }

    /// The trash folder for a file on the file system `device`.
    fn trash_folder_for(&self, path: &Path, device: u64) -> io::Result<TrashFolder> {
        let home = TrashFolder::home(&self.home_trash)?;
        if home.device()? == device {
            return Ok(home);
        }
        TrashFolder::on_file_system_of(path, device)
    }

    fn deletion_time(&self) -> io::Result<LocalDateTime> {
        let now = self.clock.now();
        LocalDateTime::from_timestamp(now, self.clock.local_offset(now)).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "the deletion time is out of range",
            )
        })
    }
}

impl<C: Clock> Trash for FreedesktopTrash<C> {
    fn move_to_trash(&self, path: &Path) -> io::Result<PathBuf> {
        let metadata = fs::symlink_metadata(path)?;
        if !metadata.is_file() {
            return Err(not_a_file(path));
        }
        let path = std::path::absolute(path)?;
        let folder = self.trash_folder_for(&path, metadata.dev())?;
        let content = trash_info::content(&folder.original_path(&path), self.deletion_time()?);
        let (name, mut info) = reserve_name(&folder, &path)?;
        let target = folder.files().join(&name);
        let moved = write_info(&mut info, &content).and_then(|()| fs::rename(&path, &target));
        if let Err(error) = moved {
            // Without the file, the info file would name a deletion that never happened.
            let _ignored = fs::remove_file(folder.info_file(&name));
            return Err(error);
        }
        Ok(target)
    }
}

/// Creates the info file of the first free name for `path`, which reserves the name
/// against other programs that trash at the same time.
fn reserve_name(folder: &TrashFolder, path: &Path) -> io::Result<(std::ffi::OsString, File)> {
    let file_name = path.file_name().ok_or_else(|| not_a_file(path))?;
    for number in 1..=u32::MAX {
        let name = trash_info::numbered_name(file_name, number);
        let info_path = folder.info_file(&name);
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&info_path)
        {
            Ok(info) if fs::symlink_metadata(folder.files().join(&name)).is_err() => {
                return Ok((name, info));
            }
            Ok(_) => fs::remove_file(&info_path)?,
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error),
        }
    }
    Err(io::Error::other("the trash has no free name"))
}

fn write_info(info: &mut File, content: &str) -> io::Result<()> {
    info.write_all(content.as_bytes())?;
    info.sync_all()
}
