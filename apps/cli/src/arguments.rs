// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The command line of `framehive` (CLI-001), built with the `clap` builder API
//! (ADR 0020 §6).

use std::ffi::OsString;
use std::path::PathBuf;

use clap::builder::PossibleValuesParser;
use clap::{Arg, ArgAction, ArgMatches, Command, value_parser};
use import::DEFAULT_NAME_TEMPLATE;

/// The values of `--on-conflict`, as the policies of IMPORT-003 are named.
const CONFLICT_POLICIES: [&str; 3] = ["add_suffix", "skip", "fail"];

/// The values of `--format` of `framehive import-log` (CLI-009).
const LOG_FORMATS: [&str; 3] = ["text", "json", "csv"];

/// What the user asked `framehive` to do.
#[derive(Debug)]
pub(crate) enum Invocation {
    /// `framehive` without a subcommand: the guided import (CLI-002).
    Guided,
    /// `framehive import`.
    Import(ImportArguments),
    /// `framehive import-log` (CLI-009).
    ImportLog(ImportLogArguments),
    /// `framehive config init` (CLI-010).
    ConfigInit,
    /// `framehive cameras` (CLI-013).
    Cameras,
}

/// The arguments of `framehive import-log` (CLI-009), checked by the parser.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ImportLogArguments {
    /// The run whose log to print; `None` lists the runs.
    pub(crate) run: Option<RunSelection>,
    /// How to print the log of one run.
    pub(crate) format: LogFormat,
}

/// The value of `<run>` of `framehive import-log`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RunSelection {
    /// A run number.
    Number(i64),
    /// `last`: the run with the highest number.
    Last,
}

/// The value of `--format` of `framehive import-log`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LogFormat {
    /// `text`, the default.
    Text,
    /// `json`.
    Json,
    /// `csv`.
    Csv,
}

/// The arguments of `framehive import`, checked by the parser.
#[derive(Debug)]
pub(crate) struct ImportArguments {
    /// The positional arguments, as given, in the order given: the sources, each a
    /// folder or `camera:…` for a camera (CLI-013, CLI-015), and last the destination
    /// when `--destination` is not given (CLI-016); empty when none was given.
    pub(crate) positionals: Vec<PathBuf>,
    /// The folder of `--destination`, as given; then every positional argument is a
    /// source (CLI-016).
    pub(crate) destination: Option<PathBuf>,
    /// The backup destinations, as given, in the order of the command line (CLI-005).
    pub(crate) backups: Vec<PathBuf>,
    /// The folder to import videos into, as given (CLI-012).
    pub(crate) video_destination: Option<PathBuf>,
    /// The video backup destinations, as given, in the order of the command line
    /// (CLI-012).
    pub(crate) video_backups: Vec<PathBuf>,
    /// The name template text, if given, not yet parsed.
    pub(crate) template: Option<String>,
    /// The video name template text, if given, not yet parsed (CLI-012).
    pub(crate) video_template: Option<String>,
    /// The job code text, if given, not yet checked (IMPORT-006).
    pub(crate) job_code: Option<String>,
    /// The conflict policy, if given.
    pub(crate) on_conflict: Option<OnConflict>,
    /// The name of the import profile, if given, not yet checked (CLI-010).
    pub(crate) profile: Option<String>,
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
    // clap rejects any other subcommand before this point.
    Ok(match matches.subcommand() {
        Some(("import", import)) => Invocation::Import(import_arguments(import)),
        Some(("config", _)) => Invocation::ConfigInit,
        Some(("cameras", _)) => Invocation::Cameras,
        Some((_, import_log)) => Invocation::ImportLog(import_log_arguments(import_log)),
        None => Invocation::Guided,
    })
}

fn command() -> Command {
    Command::new("framehive")
        .version(env!("CARGO_PKG_VERSION"))
        .about("Framehive photo library; without a subcommand, the guided import of a memory card")
        .subcommand(import_command())
        .subcommand(import_log_command())
        .subcommand(config_command())
        .subcommand(Command::new("cameras").about(
            "List the cameras and phones connected over PTP or MTP, to import from with camera:",
        ))
}

/// `framehive config`; `init` is its only subcommand so far (CLI-010).
fn config_command() -> Command {
    Command::new("config")
        .about("Manage the configuration files")
        .subcommand_required(true)
        .subcommand(
            Command::new("init")
                .about("Write the example import profile profiles/example.yaml, if it is missing"),
        )
}

fn import_log_command() -> Command {
    Command::new("import-log")
        .about("Print the import runs, or which file of one run went where, with its checksum")
        .arg(
            Arg::new("run")
                .value_name("run")
                .value_parser(run_selection)
                .help("Run number, or last; without it, the runs are listed"),
        )
        .arg(
            Arg::new("format")
                .long("format")
                .value_name("format")
                .value_parser(PossibleValuesParser::new(LOG_FORMATS))
                // `--format` applies only to the log of one run (CLI-009).
                .requires("run")
                .help("How to print the log of one run [default: text]"),
        )
}

/// Reads `<run>`: a run number or `last`.
fn run_selection(value: &str) -> Result<RunSelection, String> {
    if value == "last" {
        return Ok(RunSelection::Last);
    }
    value
        .parse::<u64>()
        .ok()
        .and_then(|number| i64::try_from(number).ok())
        .map(RunSelection::Number)
        .ok_or_else(|| "expected a run number or last".to_owned())
}

fn import_log_arguments(matches: &ArgMatches) -> ImportLogArguments {
    ImportLogArguments {
        run: matches.get_one::<RunSelection>("run").copied(),
        format: match matches.get_one::<String>("format").map(String::as_str) {
            Some("json") => LogFormat::Json,
            Some("csv") => LogFormat::Csv,
            _ => LogFormat::Text,
        },
    }
}

fn import_command() -> Command {
    Command::new("import")
        .about("Copy the photos and videos of folders, memory cards or cameras into a library folder")
        .override_usage(
            "framehive import [OPTIONS] <source>... <destination>\n       \
             framehive import [OPTIONS] --destination <folder> <source>...",
        )
        .arg(
            // Not required here, and split into sources and destination only after
            // parsing, so that a missing source or destination gets the message of
            // CLI-016 with an example instead of clap's.
            Arg::new("positionals")
                .num_args(1..)
                .value_name("source")
                .value_parser(value_parser!(PathBuf))
                .help("Folders or cameras to import from, each a folder or camera:[<model>|<port>] for a camera (./camera:… for a folder of that name), then the folder to import into, created with its parents when missing; with --destination, every one is a source"),
        )
        .arg(
            Arg::new("destination")
                .long("destination")
                .value_name("folder")
                .value_parser(value_parser!(PathBuf))
                .help("Folder to import into, created with its parents when missing; then the last folder above is a source too"),
        )
        .arg(
            Arg::new("profile")
                .long("profile")
                .value_name("name")
                .help("Import profile whose settings to use; an option given here replaces its value"),
        )
        .args(destination_arguments())
        .arg(
            Arg::new("template")
                .long("template")
                .value_name("template")
                .help(format!(
                    "Name template for the folders and name of each imported file [default: {DEFAULT_NAME_TEMPLATE}]"
                )),
        )
        .arg(
            Arg::new("video-template")
                .long("video-template")
                .value_name("template")
                .help("Name template for each imported video [default: the name template]"),
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
                .help("What to do when a target name holds a different file [default: add_suffix]"),
        )
        .arg(
            Arg::new("dry-run")
                .long("dry-run")
                .action(ArgAction::SetTrue)
                .help("Make the plan and print it, copy nothing"),
        )
}

/// The backup destinations, the video destination and the video backup destinations
/// of `framehive import` (CLI-005, CLI-012).
fn destination_arguments() -> [Arg; 3] {
    [
        Arg::new("backup")
            .long("backup")
            .value_name("folder")
            .action(ArgAction::Append)
            .value_parser(value_parser!(PathBuf))
            .help("Folder to back up each imported file to; can be given more than once"),
        Arg::new("video-destination")
            .long("video-destination")
            .value_name("folder")
            .value_parser(value_parser!(PathBuf))
            .help("Folder to import videos into [default: the destination]"),
        Arg::new("video-backup")
            .long("video-backup")
            .value_name("folder")
            .action(ArgAction::Append)
            .value_parser(value_parser!(PathBuf))
            .help("Folder to back up each imported video to, instead of the --backup folders; can be given more than once"),
    ]
}

fn import_arguments(matches: &ArgMatches) -> ImportArguments {
    ImportArguments {
        positionals: matches
            .get_many::<PathBuf>("positionals")
            .map(|sources| sources.cloned().collect())
            .unwrap_or_default(),
        destination: matches.get_one::<PathBuf>("destination").cloned(),
        backups: matches
            .get_many::<PathBuf>("backup")
            .map(|backups| backups.cloned().collect())
            .unwrap_or_default(),
        video_destination: matches.get_one::<PathBuf>("video-destination").cloned(),
        video_backups: matches
            .get_many::<PathBuf>("video-backup")
            .map(|backups| backups.cloned().collect())
            .unwrap_or_default(),
        template: matches.get_one::<String>("template").cloned(),
        video_template: matches.get_one::<String>("video-template").cloned(),
        job_code: matches.get_one::<String>("job-code").cloned(),
        on_conflict: matches
            .get_one::<String>("on-conflict")
            .map(|value| on_conflict(value)),
        profile: matches.get_one::<String>("profile").cloned(),
        dry_run: matches.get_flag("dry-run"),
    }
}

/// Maps a value that clap has already checked against [`CONFLICT_POLICIES`].
fn on_conflict(value: &str) -> OnConflict {
    match value {
        "skip" => OnConflict::Skip,
        "fail" => OnConflict::Fail,
        _ => OnConflict::AddSuffix,
    }
}
