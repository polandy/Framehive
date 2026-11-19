// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Runs the binary on a pseudo-terminal, as the guided import needs (CLI-002.1,
//! ADR 0050).

use std::fs::File;
use std::io::{Read, Write};
use std::os::fd::OwnedFd;
use std::process::{Command, ExitStatus, Stdio};
use std::thread;

use rustix::pty::{OpenptFlags, grantpt, openpt, ptsname, unlockpt};
use rustix::termios::{LocalModes, OptionalActions, OutputModes, tcgetattr, tcsetattr};

/// What a run on the terminal printed, and how it ended.
pub(crate) struct Transcript {
    /// What the terminal showed, with the line breaks the binary wrote.
    pub(crate) output: String,
    /// Standard error, a pipe of its own.
    pub(crate) errors: String,
    pub(crate) status: ExitStatus,
}

/// Runs `command` with a terminal as standard input and output, types each line of
/// `script` and then Ctrl+D, and waits until it exits.
pub(crate) fn run_on_terminal(mut command: Command, script: &[&str]) -> Transcript {
    let master = openpt(OpenptFlags::RDWR | OpenptFlags::NOCTTY).unwrap();
    grantpt(&master).unwrap();
    unlockpt(&master).unwrap();
    let terminal_name = ptsname(&master, Vec::new()).unwrap();
    let terminal = File::options()
        .read(true)
        .write(true)
        .open(terminal_name.to_str().unwrap())
        .unwrap();
    plain_mode(&terminal);
    command
        .stdin(terminal.try_clone().unwrap())
        .stdout(terminal)
        .stderr(Stdio::piped());
    let child = command.spawn().unwrap();
    // The command holds the terminal's files; the output ends only when all are closed.
    drop(command);
    let reader = read_until_closed(master.try_clone().unwrap());
    let mut input = File::from(master);
    let mut typed = script.join("\n");
    if !script.is_empty() {
        typed.push('\n');
    }
    typed.push('\u{4}');
    input.write_all(typed.as_bytes()).unwrap();
    let finished = child.wait_with_output().unwrap();
    Transcript {
        output: reader.join().unwrap(),
        errors: String::from_utf8(finished.stderr).unwrap(),
        status: finished.status,
    }
}

/// Turns off the echo of typed lines and the `\r` before each line break, so the
/// output is only what the binary wrote.
fn plain_mode(terminal: &File) {
    let mut modes = tcgetattr(terminal).unwrap();
    modes.local_modes.remove(LocalModes::ECHO);
    modes.output_modes.remove(OutputModes::OPOST);
    tcsetattr(terminal, OptionalActions::Now, &modes).unwrap();
}

/// Reads the terminal's output on a thread until the binary closes it; Linux then
/// fails the read with `EIO`.
fn read_until_closed(master: OwnedFd) -> thread::JoinHandle<String> {
    thread::spawn(move || {
        let mut output = Vec::new();
        let mut buffer = [0; 4096];
        let mut file = File::from(master);
        while let Ok(count) = file.read(&mut buffer) {
            if count == 0 {
                break;
            }
            output.extend_from_slice(&buffer[..count]);
        }
        String::from_utf8(output).unwrap()
    })
}
