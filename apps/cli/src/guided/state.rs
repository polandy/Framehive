// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The remembered state of the guided import (ADR 0025): where the state file is, its
//! plain-text format, and reading and writing it through the `FileSystem` port.

use std::ffi::OsStr;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use files::FileSystem;
use import::{JobCode, RecentJobCodes};
use naming::NameTemplate;

/// The first line of the state file, which says what it is.
const HEADER: &str = "# Framehive remembers the answers of the guided import here.";
/// The only format version this program reads and writes.
const FORMAT_VERSION: &str = "1";

/// The answers the guided import remembers between runs.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct RememberedState {
    /// The import destination, absolute.
    pub(crate) destination: Option<PathBuf>,
    /// The name template.
    pub(crate) template: Option<NameTemplate>,
    /// The job codes used before, the most recent first (IMPORT-006).
    pub(crate) job_codes: RecentJobCodes,
}

/// A state file that is read: the state, and one warning per value that was dropped.
#[derive(Debug, Default)]
pub(crate) struct LoadedState {
    /// What the file remembers, or the first-run state.
    pub(crate) state: RememberedState,
    /// What to print as `warning:` lines.
    pub(crate) warnings: Vec<String>,
}

/// Where the state file is (ADR 0025 §1): below `XDG_STATE_HOME` when it is set and
/// absolute, otherwise below `<home>/.local/state`, on Linux and macOS alike.
pub(crate) fn state_file(xdg_state_home: Option<&OsStr>, home: &Path) -> PathBuf {
    let state_home = xdg_state_home
        .map(Path::new)
        .filter(|path| path.is_absolute())
        .map_or_else(|| home.join(".local/state"), Path::to_path_buf);
    state_home.join("framehive").join("guided-import.txt")
}

/// Reads the state file. A missing file is the first-run state without a warning; a
/// file that cannot be read or understood is the first-run state with one warning.
pub(crate) fn load_state(file_system: &dyn FileSystem, path: &Path) -> LoadedState {
    let text = match read_text(file_system, path) {
        Ok(Some(text)) => text,
        Ok(None) => return LoadedState::default(),
        Err(error) => return ignored(path, &format!("cannot read it: {error}")),
    };
    match parse_state(&text) {
        Ok(loaded) => loaded,
        Err(reason) => ignored(path, &reason),
    }
}

fn ignored(path: &Path, reason: &str) -> LoadedState {
    LoadedState {
        state: RememberedState::default(),
        warnings: vec![format!(
            "the remembered answers in {} are ignored, because {reason}",
            path.display()
        )],
    }
}

fn read_text(file_system: &dyn FileSystem, path: &Path) -> io::Result<Option<String>> {
    let mut file = match file_system.open_read(path) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    };
    let mut text = String::new();
    file.read_to_string(&mut text)?;
    Ok(Some(text))
}

/// Parses the text of a state file (ADR 0025 §2).
///
/// # Errors
///
/// Returns why the whole file is ignored: another `format_version`, or a line that is
/// not understood. An invalid value only adds a warning.
pub(crate) fn parse_state(text: &str) -> Result<LoadedState, String> {
    let mut lines = text
        .lines()
        .enumerate()
        .map(|(index, line)| (index + 1, line))
        .filter(|(_, line)| !line.trim().is_empty() && !line.starts_with('#'));
    match lines.next() {
        Some((_, line)) if line == format!("format_version={FORMAT_VERSION}") => {}
        Some((number, line)) => {
            return Err(format!(
                "line {number} is {line:?}, not format_version={FORMAT_VERSION}"
            ));
        }
        None => return Err(format!("it has no format_version={FORMAT_VERSION}")),
    }
    let mut values = StoredValues::default();
    for (number, line) in lines {
        values.add(number, line)?;
    }
    Ok(values.into_state())
}

/// The values of a state file, as written, before they are checked.
#[derive(Default)]
struct StoredValues {
    destination: Option<String>,
    template: Option<String>,
    job_codes: Vec<String>,
}

impl StoredValues {
    fn add(&mut self, number: usize, line: &str) -> Result<(), String> {
        let not_understood = || format!("line {number} is not understood: {line:?}");
        let (key, value) = line.split_once('=').ok_or_else(not_understood)?;
        let slot = match key {
            "destination" => &mut self.destination,
            "template" => &mut self.template,
            "job_code" => {
                self.job_codes.push(value.to_owned());
                return Ok(());
            }
            _ => return Err(not_understood()),
        };
        if slot.is_some() {
            return Err(format!("line {number} repeats {key}"));
        }
        *slot = Some(value.to_owned());
        Ok(())
    }

    fn into_state(self) -> LoadedState {
        let mut warnings = Vec::new();
        let destination = self.destination.and_then(|text| {
            let path = PathBuf::from(&text);
            if path.is_absolute() {
                Some(path)
            } else {
                warnings.push(format!(
                    "the remembered destination {text:?} is not an absolute path; it is asked for again"
                ));
                None
            }
        });
        let template = self
            .template
            .and_then(|text| match NameTemplate::parse(&text) {
                Ok(template) => Some(template),
                Err(error) => {
                    warnings.push(format!(
                        "the remembered name template {text} is invalid ({error}); it is asked for again"
                    ));
                    None
                }
            });
        for text in &self.job_codes {
            if let Err(error) = JobCode::parse(text) {
                warnings.push(format!(
                    "the remembered job code {text:?} is dropped: {error}"
                ));
            }
        }
        LoadedState {
            state: RememberedState {
                destination,
                template,
                job_codes: RecentJobCodes::from_stored(&self.job_codes),
            },
            warnings,
        }
    }
}

/// Writes `state` in the format of ADR 0025 §2. A destination that is not valid UTF-8
/// or contains a line break is left out, and so is a template with a line break.
pub(crate) fn format_state(state: &RememberedState) -> String {
    let mut text = format!("{HEADER}\nformat_version={FORMAT_VERSION}\n");
    let destination = state.destination.as_deref().and_then(Path::to_str);
    if let Some(destination) = destination.filter(|text| fits_on_a_line(text)) {
        push_entry(&mut text, "destination", destination);
    }
    let template = state.template.as_ref().map(NameTemplate::as_str);
    if let Some(template) = template.filter(|text| fits_on_a_line(text)) {
        push_entry(&mut text, "template", template);
    }
    for job_code in state.job_codes.job_codes() {
        // A job code has no control characters, so no line break (IMPORT-006).
        push_entry(&mut text, "job_code", job_code.as_str());
    }
    text
}

/// Appends the line `key=value`.
fn push_entry(text: &mut String, key: &str, value: &str) {
    text.push_str(key);
    text.push('=');
    text.push_str(value);
    text.push('\n');
}

fn fits_on_a_line(text: &str) -> bool {
    !text.contains(['\n', '\r'])
}

/// Writes the state file whole (ADR 0025 §4): to `<file>.part`, synced, then the old
/// file is removed and the new one renamed into place. Missing folders are created.
///
/// # Errors
///
/// Returns the first error of the file system; the old file may then be gone.
pub(crate) fn save_state(
    file_system: &dyn FileSystem,
    path: &Path,
    state: &RememberedState,
) -> io::Result<()> {
    let folder = path.parent().unwrap_or(Path::new("/"));
    create_folders(file_system, folder)?;
    let mut part_name = path.as_os_str().to_owned();
    part_name.push(".part");
    let part = PathBuf::from(part_name);
    let mut file = file_system.create_file(&part)?;
    file.write_all(format_state(state).as_bytes())?;
    file.sync()?;
    drop(file);
    match file_system.remove_file(path) {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error),
    }
    file_system.rename_no_replace(&part, path)?;
    file_system.sync_folder(folder)
}

/// Creates `folder` and the folders above it that are missing, outermost first.
///
/// # Errors
///
/// Returns the first error of the file system; a folder that appears meanwhile is not
/// an error.
pub(crate) fn create_folders(file_system: &dyn FileSystem, folder: &Path) -> io::Result<()> {
    let mut missing = Vec::new();
    for ancestor in folder.ancestors() {
        if ancestor.as_os_str().is_empty() || file_system.entry(ancestor)?.is_some() {
            break;
        }
        missing.push(ancestor);
    }
    for folder in missing.into_iter().rev() {
        match file_system.create_folder(folder) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error),
        }
    }
    Ok(())
}
