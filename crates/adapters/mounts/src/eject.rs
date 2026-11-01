// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The command that ejects a device, and running it (ADR 0023 §4).

use std::ffi::{OsStr, OsString};
use std::path::Path;
use std::process::{Command, Stdio};

/// A program and its arguments that eject one device.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EjectCommand {
    /// The program, such as `udisksctl`.
    pub program: String,
    /// Its arguments.
    pub arguments: Vec<OsString>,
}

impl EjectCommand {
    /// Unmounts the block device `source`, such as `/dev/sdb1`, with udisks2 on Linux.
    ///
    /// ```
    /// use adapter_mounts::EjectCommand;
    ///
    /// let command = EjectCommand::udisksctl("/dev/sdb1".as_ref());
    /// assert_eq!(command.to_string(), "udisksctl unmount --block-device /dev/sdb1 --no-user-interaction");
    /// ```
    #[must_use]
    pub fn udisksctl(source: &OsStr) -> Self {
        EjectCommand {
            program: "udisksctl".to_owned(),
            arguments: vec![
                "unmount".into(),
                "--block-device".into(),
                source.to_owned(),
                "--no-user-interaction".into(),
            ],
        }
    }

    /// Ejects the volume mounted at `mount_point` with `diskutil` on macOS.
    #[must_use]
    pub fn diskutil(mount_point: &Path) -> Self {
        EjectCommand {
            program: "diskutil".to_owned(),
            arguments: vec!["eject".into(), mount_point.as_os_str().to_owned()],
        }
    }
}

impl std::fmt::Display for EjectCommand {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.program)?;
        for argument in &self.arguments {
            write!(formatter, " {}", argument.to_string_lossy())?;
        }
        Ok(())
    }
}

/// Runs `command` as a child process without input and waits for it.
///
/// # Errors
///
/// Returns a message if the program cannot be started, or its exit status and what it
/// wrote to standard error if it fails.
pub fn run_command(command: &EjectCommand) -> Result<(), String> {
    let output = Command::new(&command.program)
        .args(&command.arguments)
        .stdin(Stdio::null())
        .output()
        .map_err(|error| format!("cannot start {}: {error}", command.program))?;
    if output.status.success() {
        return Ok(());
    }
    let message = String::from_utf8_lossy(&output.stderr);
    let message = message.trim();
    let failure = format!("{} failed with {}", command.program, output.status);
    if message.is_empty() {
        Err(failure)
    } else {
        Err(format!("{failure}: {message}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// spec: DEVICES-001.12
    #[test]
    fn linux_unmounts_the_block_device_with_udisksctl() {
        let command = EjectCommand::udisksctl(OsStr::new("/dev/sdb1"));
        assert_eq!(command.program, "udisksctl");
        assert_eq!(
            command.arguments,
            [
                "unmount",
                "--block-device",
                "/dev/sdb1",
                "--no-user-interaction"
            ]
            .map(OsString::from)
        );
    }

    /// spec: DEVICES-001.12
    #[test]
    fn macos_ejects_the_mount_point_with_diskutil() {
        let command = EjectCommand::diskutil(Path::new("/Volumes/Untitled"));
        assert_eq!(command.to_string(), "diskutil eject /Volumes/Untitled");
    }

    /// spec: DEVICES-001.13
    #[test]
    fn a_missing_tool_cannot_be_started() {
        let command = EjectCommand {
            program: "framehive-test-no-such-tool".to_owned(),
            arguments: Vec::new(),
        };
        let message = run_command(&command).unwrap_err();
        assert!(
            message.starts_with("cannot start framehive-test-no-such-tool: "),
            "{message}"
        );
    }

    /// spec: DEVICES-001.13
    #[cfg(unix)]
    #[test]
    fn a_failing_tool_reports_its_exit_status_and_message() {
        let command = EjectCommand {
            program: "sh".to_owned(),
            arguments: ["-c", "echo 'target is busy' >&2; exit 3"]
                .map(OsString::from)
                .to_vec(),
        };
        assert_eq!(
            run_command(&command).unwrap_err(),
            "sh failed with exit status: 3: target is busy"
        );
    }

    #[cfg(unix)]
    #[test]
    fn a_failing_tool_without_message_reports_its_exit_status() {
        let command = EjectCommand {
            program: "sh".to_owned(),
            arguments: ["-c", "exit 1"].map(OsString::from).to_vec(),
        };
        assert_eq!(
            run_command(&command).unwrap_err(),
            "sh failed with exit status: 1"
        );
    }

    #[cfg(unix)]
    #[test]
    fn a_succeeding_tool_ejects() {
        let command = EjectCommand {
            program: "sh".to_owned(),
            arguments: ["-c", "exit 0"].map(OsString::from).to_vec(),
        };
        assert_eq!(run_command(&command), Ok(()));
    }
}
