// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Which trash folder a file goes to, after section "Trash directories" of the
//! freedesktop.org Trash specification.

use std::ffi::OsStr;
use std::fs::{self, DirBuilder};
use std::io;
use std::os::unix::fs::{DirBuilderExt, MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};

/// The sticky bit, which the shared `.Trash` folder of a file system must have.
const STICKY: u32 = 0o1000;

/// A trash folder with its `files` and `info` folders, which exist.
#[derive(Debug)]
pub(crate) struct TrashFolder {
    folder: PathBuf,
    /// The top folder of the file system for a trash there, which `Path` in the info
    /// file is relative to; none for the home trash, where `Path` is absolute.
    top_folder: Option<PathBuf>,
}

impl TrashFolder {
    /// The home trash at `folder`, created if it is missing.
    pub(crate) fn home(folder: &Path) -> io::Result<Self> {
        TrashFolder::create(folder.to_path_buf(), None)
    }

    /// The trash at the top folder of the file system `device` that `path` lies on:
    /// `.Trash/<uid>` if the shared `.Trash` passes the checks, `.Trash-<uid>` if not.
    pub(crate) fn on_file_system_of(path: &Path, device: u64) -> io::Result<Self> {
        let top_folder = top_folder(path, device)?;
        let user_id = user_id()?;
        TrashFolder::in_top_folder(&top_folder, user_id)
    }

    fn in_top_folder(top_folder: &Path, user_id: u32) -> io::Result<Self> {
        let shared = top_folder.join(".Trash");
        if is_usable_shared_trash(&shared) {
            let folder = shared.join(user_id.to_string());
            if let Ok(trash) = TrashFolder::create(folder, Some(top_folder.to_path_buf())) {
                return Ok(trash);
            }
        }
        let folder = top_folder.join(format!(".Trash-{user_id}"));
        TrashFolder::create(folder, Some(top_folder.to_path_buf()))
    }

    fn create(folder: PathBuf, top_folder: Option<PathBuf>) -> io::Result<Self> {
        let mut builder = DirBuilder::new();
        builder.recursive(true).mode(0o700);
        builder.create(folder.join("files"))?;
        builder.create(folder.join("info"))?;
        Ok(TrashFolder { folder, top_folder })
    }

    /// The file system this trash lies on.
    pub(crate) fn device(&self) -> io::Result<u64> {
        Ok(fs::metadata(&self.folder)?.dev())
    }

    /// The folder trashed files go to.
    pub(crate) fn files(&self) -> PathBuf {
        self.folder.join("files")
    }

    /// The info file of the trashed file `name`.
    pub(crate) fn info_file(&self, name: &OsStr) -> PathBuf {
        let mut file_name = name.to_os_string();
        file_name.push(".trashinfo");
        self.folder.join("info").join(file_name)
    }

    /// `Path` of the info file for the absolute `original`.
    pub(crate) fn original_path(&self, original: &Path) -> PathBuf {
        self.top_folder
            .as_deref()
            .and_then(|top_folder| original.strip_prefix(top_folder).ok())
            .unwrap_or(original)
            .to_path_buf()
    }
}

/// The highest folder above `path` that still lies on the file system `device`.
fn top_folder(path: &Path, device: u64) -> io::Result<PathBuf> {
    let mut top_folder = None;
    for folder in path.ancestors().skip(1) {
        if fs::metadata(folder)?.dev() != device {
            break;
        }
        top_folder = Some(folder);
    }
    top_folder
        .map(Path::to_path_buf)
        .ok_or_else(|| io::Error::other(format!("{} has no folder", path.display())))
}

/// A shared `.Trash` is a real folder, not a symbolic link, with the sticky bit set.
fn is_usable_shared_trash(shared: &Path) -> bool {
    fs::symlink_metadata(shared)
        .is_ok_and(|metadata| metadata.is_dir() && metadata.permissions().mode() & STICKY != 0)
}

/// The user id of this process; Linux shows it as the owner of `/proc/self`.
fn user_id() -> io::Result<u32> {
    Ok(fs::metadata("/proc/self")?.uid())
}

#[cfg(test)]
mod tests {
    use std::fs::Permissions;
    use std::os::unix::fs::symlink;

    use files::testing::TempDir;

    use super::*;

    const USER_ID: u32 = 1000;

    #[test]
    fn a_shared_trash_with_the_sticky_bit_gets_a_folder_per_user() {
        let top = TempDir::new().unwrap();
        fs::create_dir(top.path().join(".Trash")).unwrap();
        fs::set_permissions(top.path().join(".Trash"), Permissions::from_mode(0o1777)).unwrap();

        let trash = TrashFolder::in_top_folder(top.path(), USER_ID).unwrap();

        assert_eq!(trash.files(), top.path().join(".Trash/1000/files"));
        assert!(top.path().join(".Trash/1000/info").is_dir());
    }

    #[test]
    fn a_shared_trash_without_the_sticky_bit_is_not_used() {
        let top = TempDir::new().unwrap();
        fs::create_dir(top.path().join(".Trash")).unwrap();
        fs::set_permissions(top.path().join(".Trash"), Permissions::from_mode(0o777)).unwrap();

        let trash = TrashFolder::in_top_folder(top.path(), USER_ID).unwrap();

        assert_eq!(trash.files(), top.path().join(".Trash-1000/files"));
        assert!(!top.path().join(".Trash/1000").exists());
    }

    #[test]
    fn a_shared_trash_that_is_a_symbolic_link_is_not_used() {
        let top = TempDir::new().unwrap();
        fs::create_dir(top.path().join("elsewhere")).unwrap();
        fs::set_permissions(top.path().join("elsewhere"), Permissions::from_mode(0o1777)).unwrap();
        symlink(top.path().join("elsewhere"), top.path().join(".Trash")).unwrap();

        let trash = TrashFolder::in_top_folder(top.path(), USER_ID).unwrap();

        assert_eq!(trash.files(), top.path().join(".Trash-1000/files"));
    }

    #[test]
    fn without_a_shared_trash_the_user_trash_is_created_private() {
        let top = TempDir::new().unwrap();

        let trash = TrashFolder::in_top_folder(top.path(), USER_ID).unwrap();

        let mode = fs::metadata(trash.files()).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o700);
    }

    #[test]
    fn the_original_path_is_relative_to_the_top_folder() {
        let top = TempDir::new().unwrap();
        let trash = TrashFolder::in_top_folder(top.path(), USER_ID).unwrap();

        let original = trash.original_path(&top.path().join("2026/DSCF0001.RAF"));

        assert_eq!(original, Path::new("2026/DSCF0001.RAF"));
    }

    #[test]
    fn the_original_path_in_the_home_trash_is_absolute() {
        let home = TempDir::new().unwrap();
        let trash = TrashFolder::home(&home.path().join("Trash")).unwrap();

        let original = trash.original_path(Path::new("/media/card/DSCF0001.RAF"));

        assert_eq!(original, Path::new("/media/card/DSCF0001.RAF"));
    }

    #[test]
    fn the_top_folder_of_a_file_on_the_root_file_system_is_the_root() {
        let device = fs::metadata("/").unwrap().dev();

        assert_eq!(
            top_folder(Path::new("/missing-file"), device).unwrap(),
            Path::new("/")
        );
    }
}
