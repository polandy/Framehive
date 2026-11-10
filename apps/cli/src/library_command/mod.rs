// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! `framehive library keep-raw` and `framehive library remove-raws` (CLI-018): the Keep
//! RAW marker and Remove RAWs of the library module (LIBRARY-002) on a folder.

mod keep_raw;
mod remove_raws;
#[cfg(test)]
mod tests;

use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use adapter_blake3::Blake3Hasher;
use adapter_clock::SystemClock;
use adapter_fs::StdFileSystem;
use adapter_xmp::QuickXml;
use files::Trash;
use library::{Asset, LibraryFolder, ReadFolderError, read_folder};
use metadata::xmp::{SidecarFiles, SidecarNaming};

use crate::arguments::{KeepRawArguments, RemoveRawsArguments};
use crate::configuration_file::process_configuration;
use crate::{NOT_STARTED, report_error, write_warning};

pub(crate) use keep_raw::keep_raw;
pub(crate) use remove_raws::{Terminal, remove_raws};

/// Runs `framehive library keep-raw` (CLI-018).
pub(crate) fn run_keep_raw(arguments: &KeepRawArguments) -> ExitCode {
    let naming = match sidecar_naming() {
        Ok(naming) => naming,
        Err(message) => return not_started(&message),
    };
    let files = match absolute_paths(&arguments.files) {
        Ok(files) => files,
        Err(message) => return not_started(&message),
    };
    let ports = SystemPorts::new();
    ExitCode::from(keep_raw(
        ports.sidecars(),
        naming,
        &files,
        !arguments.off,
        &mut io::stdout().lock(),
        &mut io::stderr().lock(),
    ))
}

/// Runs `framehive library remove-raws` (CLI-018).
pub(crate) fn run_remove_raws(arguments: &RemoveRawsArguments) -> ExitCode {
    let folder = match absolute_paths(std::slice::from_ref(&arguments.folder)) {
        Ok(mut folders) => folders.remove(0),
        Err(message) => return not_started(&message),
    };
    let trash = match system_trash() {
        Ok(trash) => trash,
        Err(message) => return not_started(&message),
    };
    let ports = SystemPorts::new();
    ExitCode::from(remove_raws(
        ports.sidecars(),
        trash.as_ref(),
        &folder,
        arguments.dry_run,
        &mut Terminal {
            input: &mut io::stdin().lock(),
            output: &mut io::stdout().lock(),
            errors: &mut io::stderr().lock(),
        },
    ))
}

/// The adapters behind the sidecars of the library.
struct SystemPorts {
    files: StdFileSystem,
    hasher: Blake3Hasher,
    xml: QuickXml,
    clock: SystemClock,
}

impl SystemPorts {
    fn new() -> Self {
        SystemPorts {
            files: StdFileSystem::new(),
            hasher: Blake3Hasher,
            xml: QuickXml::new(),
            clock: SystemClock::new(),
        }
    }

    fn sidecars(&self) -> SidecarFiles<'_> {
        SidecarFiles {
            files: &self.files,
            hasher: &self.hasher,
            xml: &self.xml,
            clock: &self.clock,
        }
    }
}

/// The sidecar naming of `config.yaml` (ADR 0040).
fn sidecar_naming() -> Result<SidecarNaming, String> {
    Ok(process_configuration()?.metadata.values().sidecar_naming)
}

/// `paths` made absolute against the current folder, so every line names a file in full.
fn absolute_paths(paths: &[PathBuf]) -> Result<Vec<PathBuf>, String> {
    paths
        .iter()
        .map(|path| {
            std::path::absolute(path)
                .map_err(|error| format!("the current folder cannot be read: {error}"))
        })
        .collect()
}

#[cfg(target_os = "macos")]
#[expect(
    clippy::unnecessary_wraps,
    reason = "the same signature as the freedesktop trash, which can fail"
)]
fn system_trash() -> Result<Box<dyn Trash>, String> {
    Ok(Box::new(adapter_macos_trash::MacosTrash::new()))
}

#[cfg(not(target_os = "macos"))]
fn system_trash() -> Result<Box<dyn Trash>, String> {
    let home_trash = home_trash(
        std::env::var_os("XDG_DATA_HOME").as_deref(),
        std::env::var_os("HOME").as_deref(),
    )
    .ok_or_else(|| "the home folder is unknown: set HOME to an absolute path".to_owned())?;
    Ok(Box::new(adapter_freedesktop_trash::FreedesktopTrash::new(
        home_trash,
        SystemClock::new(),
    )))
}

#[cfg(not(target_os = "macos"))]
/// The home trash of the freedesktop.org trash: `$XDG_DATA_HOME/Trash` when it is set
/// and absolute, else `<home>/.local/share/Trash`; `None` without an absolute home.
fn home_trash(
    xdg_data_home: Option<&std::ffi::OsStr>,
    home: Option<&std::ffi::OsStr>,
) -> Option<PathBuf> {
    let data_home = xdg_data_home
        .map(Path::new)
        .filter(|path| path.is_absolute())
        .map(Path::to_path_buf)
        .or_else(|| {
            home.map(Path::new)
                .filter(|path| path.is_absolute())
                .map(|home| home.join(".local/share"))
        })?;
    Some(data_home.join("Trash"))
}

/// Reads the assets of `folder` and writes the warnings of CLI-018 ("Assets") to
/// `errors`.
fn read_library_folder(
    sidecars: SidecarFiles<'_>,
    folder: &Path,
    errors: &mut dyn Write,
) -> Result<LibraryFolder, ReadFolderError> {
    let library_folder = read_folder(sidecars, folder)?;
    for sidecar in &library_folder.unreadable {
        write_warning(errors, &format!("{sidecar}; its asset is left out"));
    }
    for ambiguous in &library_folder.ambiguous {
        let files = ambiguous.folder.join(format!("{}.*", ambiguous.base_name));
        write_warning(
            errors,
            &format!(
                "{}: more than one RAW or rendered file; each is an asset of its own",
                files.display()
            ),
        );
    }
    Ok(library_folder)
}

/// How the lines name an asset: `<folder>/<base>`.
fn asset_name(asset: &Asset) -> String {
    asset
        .id()
        .with_file_name(asset.base_name())
        .display()
        .to_string()
}

/// Writes `line` to `output`, which stands for standard output.
fn write_line(output: &mut dyn Write, line: &str) {
    // Nothing more can be reported when standard output is gone.
    let _ = writeln!(output, "{line}");
}

fn not_started(message: &str) -> ExitCode {
    report_error(message);
    ExitCode::from(NOT_STARTED)
}
