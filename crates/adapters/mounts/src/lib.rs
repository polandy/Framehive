// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The `mounts` adapter of Framehive: it implements the [`devices::DeviceSource`] port
//! over the file systems the desktop has mounted (DEVICES-001, ADR 0023 §2, §4). Only
//! apps depend on it. See `README.md`.
//!
//! ```no_run
//! use adapter_mounts::MountsDeviceSource;
//! use devices::DeviceSource;
//!
//! for device in MountsDeviceSource::new().list()? {
//!     println!("{device}");
//! }
//! # Ok::<(), devices::ListError>(())
//! ```

#![forbid(unsafe_code)]

mod eject;
mod mountinfo;

use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

use devices::{Device, DeviceSource, EjectError, ListError};

pub use eject::{EjectCommand, run_command};

/// Runs an eject command; [`run_command`] on the real system. A seam, so that tests
/// never start the system's eject tools.
pub type RunCommand = Box<dyn Fn(&EjectCommand) -> Result<(), String> + Send + Sync>;

/// The folder at the top level of a mount that makes it a card (DCF).
const DCIM: &str = "DCIM";

/// A [`DeviceSource`] over mounted file systems: on Linux the mounts of
/// `/proc/self/mountinfo` below `/run/media/` and `/media/`, on macOS the folders
/// directly in `/Volumes/`. Only mounts with a `DCIM` folder are devices.
pub struct MountsDeviceSource {
    finder: Finder,
    run: RunCommand,
}

/// Where the considered mounts come from, and how a device is ejected.
#[derive(Debug, Clone)]
enum Finder {
    Linux {
        mountinfo: PathBuf,
        considered: Vec<PathBuf>,
    },
    MacOs {
        volumes: PathBuf,
    },
}

/// A mount that is considered, and what the eject command needs of it.
struct Considered {
    mount_point: PathBuf,
    /// The mount source on Linux, such as `/dev/sdb1`.
    source: Option<OsString>,
}

impl MountsDeviceSource {
    /// The devices of this system, read the way of its platform: macOS on macOS,
    /// Linux everywhere else.
    #[must_use]
    pub fn new() -> Self {
        #[cfg(target_os = "macos")]
        let source = MountsDeviceSource::macos("/Volumes");
        #[cfg(not(target_os = "macos"))]
        let source = MountsDeviceSource::linux(
            "/proc/self/mountinfo",
            vec![PathBuf::from("/run/media"), PathBuf::from("/media")],
        );
        source
    }

    /// The Linux way: the mounts listed in the `mountinfo` file whose mount point is
    /// below one of the `considered` folders; ejected with `udisksctl`.
    #[must_use]
    pub fn linux(mountinfo: impl Into<PathBuf>, considered: Vec<PathBuf>) -> Self {
        MountsDeviceSource::with_finder(Finder::Linux {
            mountinfo: mountinfo.into(),
            considered,
        })
    }

    /// The macOS way: the folders directly in `volumes`, without symbolic links;
    /// ejected with `diskutil`.
    #[must_use]
    pub fn macos(volumes: impl Into<PathBuf>) -> Self {
        MountsDeviceSource::with_finder(Finder::MacOs {
            volumes: volumes.into(),
        })
    }

    /// The same source, with eject commands run by `run` instead of [`run_command`].
    #[must_use]
    pub fn with_runner(self, run: RunCommand) -> Self {
        MountsDeviceSource { run, ..self }
    }

    fn with_finder(finder: Finder) -> Self {
        MountsDeviceSource {
            finder,
            run: Box::new(run_command),
        }
    }

    /// The considered mounts that are cards, sorted by mount point and each once.
    fn cards(&self) -> Result<Vec<Considered>, ListError> {
        let mut cards: Vec<Considered> = match &self.finder {
            Finder::Linux {
                mountinfo,
                considered,
            } => linux_mounts(mountinfo, considered)?,
            Finder::MacOs { volumes } => macos_volumes(volumes)?,
        };
        cards.retain(|mount| has_dcim_folder(&mount.mount_point));
        // A later line of mountinfo is mounted over an earlier one at the same place.
        cards.reverse();
        cards.sort_by(|left, right| {
            path_bytes(&left.mount_point).cmp(path_bytes(&right.mount_point))
        });
        cards.dedup_by(|later, earlier| later.mount_point == earlier.mount_point);
        Ok(cards)
    }

    fn eject_command(&self, card: &Considered) -> EjectCommand {
        match (&self.finder, &card.source) {
            (Finder::Linux { .. }, Some(source)) => EjectCommand::udisksctl(source),
            _ => EjectCommand::diskutil(&card.mount_point),
        }
    }
}

impl Default for MountsDeviceSource {
    fn default() -> Self {
        MountsDeviceSource::new()
    }
}

impl std::fmt::Debug for MountsDeviceSource {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("MountsDeviceSource")
            .field("finder", &self.finder)
            .finish_non_exhaustive()
    }
}

impl DeviceSource for MountsDeviceSource {
    fn list(&self) -> Result<Vec<Device>, ListError> {
        Ok(self
            .cards()?
            .into_iter()
            .map(|card| Device::at(card.mount_point))
            .collect())
    }

    fn eject(&self, device: &Device) -> Result<(), EjectError> {
        let cards = self.cards()?;
        let Some(card) = cards
            .iter()
            .find(|card| card.mount_point == device.mount_point())
        else {
            return Err(EjectError::NotPresent {
                device: device.clone(),
            });
        };
        (self.run)(&self.eject_command(card)).map_err(|message| EjectError::Failed {
            device: device.clone(),
            message,
        })
    }
}

/// The mounts of `mountinfo` strictly below one of the `considered` folders.
fn linux_mounts(mountinfo: &Path, considered: &[PathBuf]) -> Result<Vec<Considered>, ListError> {
    let text = fs::read(mountinfo)
        .map_err(|error| ListError::new(mountinfo.display().to_string(), error))?;
    Ok(mountinfo::parse(&text)
        .into_iter()
        .filter(|mount| is_below_any(&mount.mount_point, considered))
        .map(|mount| Considered {
            mount_point: mount.mount_point,
            source: Some(mount.source),
        })
        .collect())
}

fn is_below_any(mount_point: &Path, folders: &[PathBuf]) -> bool {
    folders
        .iter()
        .any(|folder| mount_point != folder && mount_point.starts_with(folder))
}

/// The folders directly in `volumes`; symbolic links, such as the startup disk, and
/// entries that cannot be examined are skipped.
fn macos_volumes(volumes: &Path) -> Result<Vec<Considered>, ListError> {
    let entries = fs::read_dir(volumes)
        .map_err(|error| ListError::new(volumes.display().to_string(), error))?;
    Ok(entries
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
        .map(|entry| Considered {
            mount_point: entry.path(),
            source: None,
        })
        .collect())
}

/// Whether `mount_point` has a folder `DCIM`; one that cannot be examined has none.
fn has_dcim_folder(mount_point: &Path) -> bool {
    fs::metadata(mount_point.join(DCIM)).is_ok_and(|metadata| metadata.is_dir())
}

fn path_bytes(path: &Path) -> &[u8] {
    path.as_os_str().as_encoded_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn considered() -> Vec<PathBuf> {
        vec![PathBuf::from("/run/media"), PathBuf::from("/media")]
    }

    /// spec: DEVICES-001.4
    #[test]
    fn only_mounts_below_the_removable_locations_are_considered() {
        for mount_point in [
            "/",
            "/home",
            "/mnt/nas",
            "/run/media",
            "/media",
            "/run/mediax/a",
            "/run/user/1000",
        ] {
            assert!(
                !is_below_any(Path::new(mount_point), &considered()),
                "{mount_point}"
            );
        }
        for mount_point in [
            "/run/media/anna/EOS_DIGITAL",
            "/media/anna/EOS_DIGITAL",
            "/media/CARD",
        ] {
            assert!(
                is_below_any(Path::new(mount_point), &considered()),
                "{mount_point}"
            );
        }
    }

    #[test]
    fn the_real_system_is_read_from_its_own_mount_table() {
        let source = MountsDeviceSource::new();
        #[cfg(target_os = "macos")]
        assert!(
            matches!(&source.finder, Finder::MacOs { volumes } if volumes == Path::new("/Volumes"))
        );
        #[cfg(not(target_os = "macos"))]
        assert!(matches!(
            &source.finder,
            Finder::Linux { mountinfo, considered: folders }
                if mountinfo == Path::new("/proc/self/mountinfo") && *folders == considered()
        ));
    }
}
