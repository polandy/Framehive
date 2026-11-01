// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Fake mount tables in temporary folders, shared by the tests of `adapter-mounts`.
//!
//! The adapter may depend only on its port's crate (architecture §3), so it cannot use
//! `files::testing::TempDir` and has its own small temporary folder.

#![allow(clippy::unwrap_used, reason = "test helpers may panic on bad fixtures")]
#![allow(dead_code, reason = "each test binary uses only some of the helpers")]

use std::fs;
use std::path::{Path, PathBuf};
use std::process;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use adapter_mounts::{EjectCommand, MountsDeviceSource};
use devices::DeviceSource;
use devices::testing::{DcimFolder, DeviceSourceUnderTest};

static NEXT_NUMBER: AtomicU64 = AtomicU64::new(0);

/// A new, empty folder in the system's temporary folder, removed when dropped.
pub(crate) struct TempDir {
    path: PathBuf,
}

impl TempDir {
    pub(crate) fn new() -> Self {
        let number = NEXT_NUMBER.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("framehive-mounts-test-{}-{number}", process::id()));
        // A folder left behind by an earlier process with the same id is stale.
        let _ = fs::remove_dir_all(&path);
        fs::create_dir(&path).unwrap();
        TempDir { path }
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

/// Creates a mounted file system at `mount_point` with `DCIM` as given.
pub(crate) fn make_mount(mount_point: &Path, dcim: DcimFolder) {
    fs::create_dir_all(mount_point).unwrap();
    match dcim {
        DcimFolder::Folder => fs::create_dir_all(mount_point.join("DCIM")).unwrap(),
        DcimFolder::File => fs::write(mount_point.join("DCIM"), b"").unwrap(),
        DcimFolder::Missing => {}
    }
}

/// A `mountinfo` line as the kernel writes it, with space, tab, newline and
/// backslash escaped as octal.
pub(crate) fn mountinfo_line(id: u32, mount_point: &Path, source: &str) -> String {
    format!(
        "{id} 29 8:{id} / {} rw,nosuid,nodev,relatime shared:{id} - vfat {} rw,uid=1000\n",
        escape(&mount_point.display().to_string()),
        escape(source)
    )
}

fn escape(field: &str) -> String {
    field
        .chars()
        .map(|character| match character {
            ' ' | '\t' | '\n' | '\\' => format!("\\{:03o}", u32::from(character)),
            other => other.to_string(),
        })
        .collect()
}

/// Removes the lines of the file at `path` that contain `text`.
fn remove_lines(path: &Path, text: &str) {
    let kept: String = fs::read_to_string(path)
        .unwrap()
        .lines()
        .filter(|line| !line.contains(text))
        .flat_map(|line| [line, "\n"])
        .collect();
    fs::write(path, kept).unwrap();
}

/// Whether eject commands fail, and with which message.
type EjectFailure = Arc<Mutex<Option<String>>>;

/// A Linux system in a temporary folder: a fake `mountinfo` file and the removable
/// location `run/media/anna`. A successful eject removes the line of its device.
pub(crate) struct LinuxSystem {
    pub(crate) root: TempDir,
    source: MountsDeviceSource,
    eject_failure: EjectFailure,
    next_id: u32,
}

impl LinuxSystem {
    pub(crate) fn new() -> Self {
        let root = TempDir::new();
        let mountinfo = root.path().join("mountinfo");
        fs::write(&mountinfo, "").unwrap();
        let eject_failure = EjectFailure::default();
        let source = MountsDeviceSource::linux(
            &mountinfo,
            vec![root.path().join("run/media"), root.path().join("media")],
        )
        .with_runner(udisksctl_double(mountinfo, Arc::clone(&eject_failure)));
        LinuxSystem {
            root,
            source,
            eject_failure,
            next_id: 100,
        }
    }

    pub(crate) fn mountinfo(&self) -> PathBuf {
        self.root.path().join("mountinfo")
    }

    /// Adds a mount at `mount_point`, which may lie anywhere, to the mount table.
    pub(crate) fn mount(&mut self, mount_point: &Path, source: &str, dcim: DcimFolder) {
        make_mount(mount_point, dcim);
        let mut text = fs::read_to_string(self.mountinfo()).unwrap();
        text.push_str(&mountinfo_line(self.next_id, mount_point, source));
        fs::write(self.mountinfo(), text).unwrap();
        self.next_id += 1;
    }
}

/// Stands in for `udisksctl unmount --block-device <source>`: it removes the lines of
/// `source` from `mountinfo`, or fails with the configured message.
fn udisksctl_double(mountinfo: PathBuf, failure: EjectFailure) -> adapter_mounts::RunCommand {
    Box::new(move |command: &EjectCommand| {
        if let Some(message) = failure.lock().unwrap().clone() {
            return Err(message);
        }
        assert_eq!(command.program, "udisksctl");
        let source = escape(&command.arguments[2].to_string_lossy());
        remove_lines(&mountinfo, &format!(" - vfat {source} "));
        Ok(())
    })
}

impl DeviceSourceUnderTest for LinuxSystem {
    fn source(&self) -> &dyn DeviceSource {
        &self.source
    }

    fn insert(&mut self, name: &str, dcim: DcimFolder) -> PathBuf {
        let mount_point = self.root.path().join("run/media/anna").join(name);
        let source = format!("/dev/sd{}1", self.next_id);
        self.mount(&mount_point, &source, dcim);
        mount_point
    }

    fn remove(&mut self, mount_point: &Path) {
        let line_of_mount = format!(" / {} ", escape(&mount_point.display().to_string()));
        remove_lines(&self.mountinfo(), &line_of_mount);
        fs::remove_dir_all(mount_point).unwrap();
    }

    fn break_listing(&mut self) -> String {
        fs::remove_file(self.mountinfo()).unwrap();
        self.mountinfo().display().to_string()
    }

    fn fail_ejects(&mut self, message: &str) {
        *self.eject_failure.lock().unwrap() = Some(message.to_owned());
    }
}

/// A macOS system in a temporary folder with the folder `Volumes`. A successful eject
/// removes the volume's folder.
pub(crate) struct MacOsSystem {
    pub(crate) root: TempDir,
    source: MountsDeviceSource,
    eject_failure: EjectFailure,
}

impl MacOsSystem {
    pub(crate) fn new() -> Self {
        let root = TempDir::new();
        let volumes = root.path().join("Volumes");
        fs::create_dir(&volumes).unwrap();
        let eject_failure = EjectFailure::default();
        let source = MountsDeviceSource::macos(&volumes)
            .with_runner(diskutil_double(Arc::clone(&eject_failure)));
        MacOsSystem {
            root,
            source,
            eject_failure,
        }
    }

    pub(crate) fn volumes(&self) -> PathBuf {
        self.root.path().join("Volumes")
    }
}

/// Stands in for `diskutil eject <mount point>`: it removes the volume's folder, or
/// fails with the configured message.
fn diskutil_double(failure: EjectFailure) -> adapter_mounts::RunCommand {
    Box::new(move |command: &EjectCommand| {
        if let Some(message) = failure.lock().unwrap().clone() {
            return Err(message);
        }
        assert_eq!(command.program, "diskutil");
        fs::remove_dir_all(&command.arguments[1]).unwrap();
        Ok(())
    })
}

impl DeviceSourceUnderTest for MacOsSystem {
    fn source(&self) -> &dyn DeviceSource {
        &self.source
    }

    fn insert(&mut self, name: &str, dcim: DcimFolder) -> PathBuf {
        let mount_point = self.volumes().join(name);
        make_mount(&mount_point, dcim);
        mount_point
    }

    fn remove(&mut self, mount_point: &Path) {
        fs::remove_dir_all(mount_point).unwrap();
    }

    fn break_listing(&mut self) -> String {
        fs::remove_dir_all(self.volumes()).unwrap();
        self.volumes().display().to_string()
    }

    fn fail_ejects(&mut self, message: &str) {
        *self.eject_failure.lock().unwrap() = Some(message.to_owned());
    }
}
