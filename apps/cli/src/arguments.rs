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
    /// The folder to import from, as given.
    pub(crate) source: PathBuf,
    /// The folder to import into, as given.
    pub(crate) destination: PathBuf,
    /// The backup destinations, as given, in the order of the command line (CLI-005).
    pub(crate) backups: Vec<PathBuf>,
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
    // clap rejects any other subcommand before this point.
    Ok(match matches.subcommand() {
        Some(("import", import)) => Invocation::Import(import_arguments(import)),
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
            Arg::new("backup")
                .long("backup")
                .value_name("folder")
                .action(ArgAction::Append)
                .value_parser(value_parser!(PathBuf))
                .help("Folder to back up each imported file to; can be given more than once"),
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
        backups: matches
            .get_many::<PathBuf>("backup")
            .map(|backups| backups.cloned().collect())
            .unwrap_or_default(),
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
