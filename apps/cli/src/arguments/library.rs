// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! `framehive library` with `keep-raw` and `remove-raws` (CLI-018).

use std::path::PathBuf;

use clap::{Arg, ArgAction, ArgMatches, Command, value_parser};

use super::Invocation;

/// The arguments of `framehive library keep-raw` (CLI-018).
#[derive(Debug)]
pub(crate) struct KeepRawArguments {
    /// The files whose assets to mark, as given.
    pub(crate) files: Vec<PathBuf>,
    /// `--off`: unmark them.
    pub(crate) off: bool,
}

/// The arguments of `framehive library remove-raws` (CLI-018).
#[derive(Debug)]
pub(crate) struct RemoveRawsArguments {
    /// The folder, as given.
    pub(crate) folder: PathBuf,
    /// `--dry-run`: print the summary and the files, move nothing.
    pub(crate) dry_run: bool,
}

/// `framehive library` with `keep-raw` and `remove-raws` (CLI-018).
pub(super) fn library_command() -> Command {
    Command::new("library")
        .about("Work on the photos already in the library")
        .subcommand_required(true)
        .subcommand(
            Command::new("keep-raw")
                .about("Mark the assets of these files Keep RAW, so Remove RAWs keeps their RAW")
                .arg(
                    Arg::new("files")
                        .required(true)
                        .num_args(1..)
                        .value_name("file")
                        .value_parser(value_parser!(PathBuf))
                        .help("Photos, videos or sidecars of the assets to mark"),
                )
                .arg(
                    Arg::new("off")
                        .long("off")
                        .action(ArgAction::SetTrue)
                        .help("Unmark them instead"),
                ),
        )
        .subcommand(
            Command::new("remove-raws")
                .about("Move the RAW of every RAW + JPEG asset of a folder to the trash, unless it is marked Keep RAW or rejected")
                .arg(
                    Arg::new("folder")
                        .required(true)
                        .value_name("folder")
                        .value_parser(value_parser!(PathBuf))
                        .help("Folder whose assets to look at, without its subfolders"),
                )
                .arg(
                    Arg::new("dry-run")
                        .long("dry-run")
                        .action(ArgAction::SetTrue)
                        .help("Print the summary and the files, move nothing"),
                ),
        )
}

pub(super) fn library_invocation(matches: &ArgMatches) -> Invocation {
    if let Some(keep_raw) = matches.subcommand_matches("keep-raw") {
        return Invocation::KeepRaw(KeepRawArguments {
            files: keep_raw
                .get_many::<PathBuf>("files")
                .map(|files| files.cloned().collect())
                .unwrap_or_default(),
            off: keep_raw.get_flag("off"),
        });
    }
    // clap requires a subcommand, so this is `remove-raws`.
    let remove_raws = matches.subcommand_matches("remove-raws");
    Invocation::RemoveRaws(RemoveRawsArguments {
        folder: remove_raws
            .and_then(|remove_raws| remove_raws.get_one::<PathBuf>("folder"))
            .cloned()
            .unwrap_or_default(),
        dry_run: remove_raws.is_some_and(|remove_raws| remove_raws.get_flag("dry-run")),
    })
}
