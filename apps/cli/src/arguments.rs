// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The command line of `framehive` (CLI-001), built with the `clap` builder API
//! (ADR 0020 §6).

use std::ffi::OsString;
use std::path::PathBuf;

use clap::builder::PossibleValuesParser;
use clap::{Arg, ArgAction, ArgMatches, Command, value_parser};

/// The name template of CLI-001 when `--template` is not given.
const DEFAULT_TEMPLATE: &str = "{date:%Y}/{date:%Y-%m-%d}/{name}.{ext}";

/// The values of `--on-conflict`, as the policies of IMPORT-003 are named.
const CONFLICT_POLICIES: [&str; 3] = ["add_suffix", "skip", "fail"];

/// What the user asked `framehive` to do.
#[derive(Debug)]
pub(crate) enum Invocation {
    /// `framehive import`.
    Import(ImportArguments),
}

/// The arguments of `framehive import`, checked by the parser.
#[derive(Debug)]
pub(crate) struct ImportArguments {
    /// The folder to import from, as given.
    pub(crate) source: PathBuf,
    /// The folder to import into, as given.
    pub(crate) destination: PathBuf,
    /// The name template text, not yet parsed.
    pub(crate) template: String,
    /// The job code text, if given, not yet checked (IMPORT-006).
    pub(crate) job_code: Option<String>,
    /// The conflict policy.
    pub(crate) on_conflict: OnConflict,
    /// Plan and print, copy nothing.
    pub(crate) dry_run: bool,
}

/// The value of `--on-conflict`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OnConflict {
    /// `add_suffix`.
    AddSuffix,
    /// `skip`.
    Skip,
    /// `fail`.
    Fail,
}

/// Parses the command line, including the program name.
///
/// # Errors
///
/// Returns the `clap` error for invalid arguments, and also for `--help` and
/// `--version`, whose text the error carries.
pub(crate) fn parse(
    arguments: impl IntoIterator<Item = OsString>,
) -> Result<Invocation, clap::Error> {
    let matches = command().try_get_matches_from(arguments)?;
    match matches.subcommand() {
        Some(("import", import)) => Ok(Invocation::Import(import_arguments(import))),
        // `subcommand_required` makes clap reject every other case.
        _ => Err(command().error(
            clap::error::ErrorKind::MissingSubcommand,
            "a subcommand is required",
        )),
    }
}

fn command() -> Command {
    Command::new("framehive")
        .version(env!("CARGO_PKG_VERSION"))
        .about("Framehive photo library")
        .subcommand_required(true)
        .subcommand(import_command())
}

fn import_command() -> Command {
    Command::new("import")
        .about("Copy the photos and videos of a folder or memory card into a library folder")
        .arg(
            Arg::new("source")
                .required(true)
                .value_parser(value_parser!(PathBuf))
                .help("Folder to import from"),
        )
        .arg(
            Arg::new("destination")
                .required(true)
                .value_parser(value_parser!(PathBuf))
                .help("Folder to import into; created with its parents when missing"),
        )
        .arg(
            Arg::new("template")
                .long("template")
                .value_name("template")
                .default_value(DEFAULT_TEMPLATE)
                .help("Name template for the folders and name of each imported file"),
        )
        .arg(
            Arg::new("job-code")
                .long("job-code")
                .value_name("text")
                .help("Job code that the {job} token of the name template inserts"),
        )
        .arg(
            Arg::new("on-conflict")
                .long("on-conflict")
                .value_name("policy")
                .value_parser(PossibleValuesParser::new(CONFLICT_POLICIES))
                .default_value("add_suffix")
                .help("What to do when a target name holds a different file"),
        )
        .arg(
            Arg::new("dry-run")
                .long("dry-run")
                .action(ArgAction::SetTrue)
                .help("Make the plan and print it, copy nothing"),
        )
}

fn import_arguments(matches: &ArgMatches) -> ImportArguments {
    ImportArguments {
        source: path(matches, "source"),
        destination: path(matches, "destination"),
        template: matches
            .get_one::<String>("template")
            .cloned()
            .unwrap_or_else(|| DEFAULT_TEMPLATE.to_owned()),
        job_code: matches.get_one::<String>("job-code").cloned(),
        on_conflict: on_conflict(matches.get_one::<String>("on-conflict").map(String::as_str)),
        dry_run: matches.get_flag("dry-run"),
    }
}

/// Reads a required path; clap has already rejected a missing one.
fn path(matches: &ArgMatches, id: &str) -> PathBuf {
    matches.get_one::<PathBuf>(id).cloned().unwrap_or_default()
}

/// Maps a value that clap has already checked against [`CONFLICT_POLICIES`].
fn on_conflict(value: Option<&str>) -> OnConflict {
    match value {
        Some("skip") => OnConflict::Skip,
        Some("fail") => OnConflict::Fail,
        _ => OnConflict::AddSuffix,
    }
}
