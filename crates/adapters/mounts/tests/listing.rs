// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Which mounts the adapter lists and how it ejects them, on fake Linux and macOS
//! systems in temporary folders (DEVICES-001.4–7, .12–14).

#![allow(clippy::unwrap_used, reason = "test helpers may panic on bad fixtures")]

mod support;

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use adapter_mounts::{EjectCommand, MountsDeviceSource};
use devices::testing::{DcimFolder, DeviceSourceUnderTest};
use devices::{Device, DeviceSource, EjectError};
use support::{LinuxSystem, MacOsSystem, TempDir, make_mount};

fn mount_points(source: &dyn DeviceSource) -> Vec<PathBuf> {
    source
        .list()
        .unwrap()
        .iter()
        .map(|device| device.mount_point().to_path_buf())
        .collect()
}

/// spec: DEVICES-001.4
#[test]
fn linux_lists_only_mounts_below_the_removable_locations() {
    let mut system = LinuxSystem::new();
    let root = system.root.path().to_path_buf();
    for (mount_point, source) in [
        (root.clone(), "/dev/nvme0n1p2"),
        (root.join("home"), "/dev/nvme0n1p3"),
        (root.join("mnt/nas"), "nas:/photos"),
        (root.join("run/media/anna/EOS_DIGITAL"), "/dev/sdb1"),
        (root.join("media/anna/Untitled"), "/dev/sdc1"),
    ] {
        system.mount(&mount_point, source, DcimFolder::Folder);
    }

    assert_eq!(
        mount_points(system.source()),
        [
            root.join("media/anna/Untitled"),
            root.join("run/media/anna/EOS_DIGITAL")
        ]
    );
}

/// spec: DEVICES-001.5
#[test]
fn linux_decodes_escaped_mount_points() {
    let mut system = LinuxSystem::new();
    let mount_point = system.root.path().join("run/media/anna/NO NAME");
    system.mount(&mount_point, "/dev/sdb1", DcimFolder::Folder);
    let line = fs::read_to_string(system.mountinfo()).unwrap();
    assert!(line.contains(r"NO\040NAME"), "{line}");

    let devices = system.source().list().unwrap();

    assert_eq!(devices, [Device::at(&mount_point)]);
    assert_eq!(devices[0].name(), "NO NAME");
}

/// A card that is mounted twice at one place is listed once, and the eject unmounts
/// the later mount, which hides the earlier one.
#[test]
fn linux_lists_a_mount_point_once_and_ejects_its_top_mount() {
    let mut system = LinuxSystem::new();
    let mount_point = system.root.path().join("run/media/anna/EOS_DIGITAL");
    system.mount(&mount_point, "/dev/sdb1", DcimFolder::Folder);
    system.mount(&mount_point, "/dev/sdc1", DcimFolder::Folder);
    let (commands, run) = recorder();
    let source =
        MountsDeviceSource::linux(system.mountinfo(), vec![system.root.path().join("run")])
            .with_runner(run);

    assert_eq!(mount_points(&source), std::slice::from_ref(&mount_point));
    source.eject(&Device::at(&mount_point)).unwrap();
    assert_eq!(
        *commands.lock().unwrap(),
        [EjectCommand::udisksctl("/dev/sdc1".as_ref())]
    );
}

/// spec: DEVICES-001.6
#[cfg(unix)]
#[test]
fn macos_lists_folders_in_volumes_and_skips_symbolic_links() {
    let system = MacOsSystem::new();
    let untitled = system.volumes().join("Untitled");
    make_mount(&untitled, DcimFolder::Folder);
    // The startup disk links to `/`; a linked folder with DCIM shows that the link
    // itself is skipped, not only a target without DCIM.
    let startup_disk = system.root.path().join("startup disk");
    make_mount(&startup_disk, DcimFolder::Folder);
    std::os::unix::fs::symlink(&startup_disk, system.volumes().join("Macintosh HD")).unwrap();
    fs::write(system.volumes().join(".DS_Store"), b"").unwrap();

    assert_eq!(mount_points(system.source()), [untitled]);
}

/// spec: DEVICES-001.7
#[test]
fn linux_names_the_mount_table_that_cannot_be_read() {
    let folder = TempDir::new();
    let mountinfo = folder.path().join("mountinfo");
    let source = MountsDeviceSource::linux(&mountinfo, vec![folder.path().join("media")]);

    let error = source.list().unwrap_err();

    assert_eq!(error.what(), mountinfo.display().to_string());
    assert!(
        error
            .to_string()
            .starts_with(&format!("cannot read {}: ", mountinfo.display()))
    );
}

/// spec: DEVICES-001.7
#[test]
fn macos_names_the_volumes_folder_that_cannot_be_read() {
    let folder = TempDir::new();
    let volumes = folder.path().join("Volumes");

    let error = MountsDeviceSource::macos(&volumes).list().unwrap_err();

    assert_eq!(error.what(), volumes.display().to_string());
}

/// Records the eject commands and lets them succeed.
fn recorder() -> (Arc<Mutex<Vec<EjectCommand>>>, adapter_mounts::RunCommand) {
    let commands = Arc::new(Mutex::new(Vec::new()));
    let recorded = Arc::clone(&commands);
    let run = Box::new(move |command: &EjectCommand| {
        recorded.lock().unwrap().push(command.clone());
        Ok(())
    });
    (commands, run)
}

/// spec: DEVICES-001.12
#[test]
fn linux_ejects_the_mount_source_with_udisksctl() {
    let mut system = LinuxSystem::new();
    let mount_point = system.root.path().join("run/media/anna/EOS_DIGITAL");
    system.mount(&mount_point, "/dev/sdb1", DcimFolder::Folder);
    let (commands, run) = recorder();
    let source = MountsDeviceSource::linux(
        system.mountinfo(),
        vec![system.root.path().join("run/media")],
    )
    .with_runner(run);

    source.eject(&Device::at(&mount_point)).unwrap();

    assert_eq!(
        *commands.lock().unwrap(),
        [EjectCommand::udisksctl("/dev/sdb1".as_ref())]
    );
}

/// spec: DEVICES-001.12
#[test]
fn macos_ejects_the_mount_point_with_diskutil() {
    let system = MacOsSystem::new();
    let untitled = system.volumes().join("Untitled");
    make_mount(&untitled, DcimFolder::Folder);
    let (commands, run) = recorder();
    let source = MountsDeviceSource::macos(system.volumes()).with_runner(run);

    source.eject(&Device::at(&untitled)).unwrap();

    assert_eq!(
        *commands.lock().unwrap(),
        [EjectCommand::diskutil(&untitled)]
    );
}

/// spec: DEVICES-001.13
#[test]
fn an_eject_tool_that_cannot_be_started_fails_the_eject_and_keeps_the_card() {
    let system = MacOsSystem::new();
    let untitled = system.volumes().join("Untitled");
    make_mount(&untitled, DcimFolder::Folder);
    let source = MountsDeviceSource::macos(system.volumes()).with_runner(Box::new(|command| {
        let missing = EjectCommand {
            program: "framehive-test-no-such-tool".to_owned(),
            arguments: command.arguments.clone(),
        };
        adapter_mounts::run_command(&missing)
    }));
    let card = Device::at(&untitled);

    let error = source.eject(&card).unwrap_err();

    let EjectError::Failed { device, message } = &error else {
        panic!("expected a failed eject, got {error}");
    };
    assert_eq!(*device, card);
    assert!(
        message.starts_with("cannot start framehive-test-no-such-tool: "),
        "{message}"
    );
    assert!(
        error.to_string().contains(&untitled.display().to_string()),
        "{error}"
    );
    assert_eq!(source.list().unwrap(), [card]);
}

/// spec: DEVICES-001.14
#[test]
fn ejecting_a_device_that_is_not_listed_says_it_is_not_present() {
    let mut system = LinuxSystem::new();
    let backup = system.insert("BACKUP", DcimFolder::Missing);
    for device in [
        Device::at(&backup),
        Device::at(Path::new("/run/media/anna/EOS_DIGITAL")),
    ] {
        let error = system.source().eject(&device).unwrap_err();
        assert!(
            matches!(&error, EjectError::NotPresent { device: named } if *named == device),
            "{error}"
        );
    }
}
