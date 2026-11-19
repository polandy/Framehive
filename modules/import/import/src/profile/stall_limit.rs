// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The stall limit of a profile, `source_stall_limit_seconds` (IMPORT-022).

use std::time::Duration;

use config::{ConfigError, ConfigFile, Position, Problem, YamlNode, YamlParser, YamlValue};

use crate::profile_file_type::{STALL_LIMIT_KEY, STALL_LIMIT_VERSION};
use crate::source_guard::DEFAULT_SOURCE_STALL_LIMIT;

/// The fewest and the most seconds of the stall limit (IMPORT-022).
const STALL_LIMIT_SECONDS: std::ops::RangeInclusive<i64> = 1..=3600;

/// The stall limit, which profiles before version 3 do not know (IMPORT-022).
pub(super) fn source_stall_limit(config: &ConfigFile) -> Result<Duration, ConfigError> {
    let Some(seconds) = config.values().whole_number(STALL_LIMIT_KEY.name()) else {
        return Ok(DEFAULT_SOURCE_STALL_LIMIT);
    };
    if config.schema_version() < STALL_LIMIT_VERSION {
        let name = STALL_LIMIT_KEY.name();
        return Err(config.error_at(
            seconds.position,
            format!("{name} needs schema_version {STALL_LIMIT_VERSION}"),
        ));
    }
    match u64::try_from(seconds.value) {
        Ok(whole) if STALL_LIMIT_SECONDS.contains(&seconds.value) => Ok(Duration::from_secs(whole)),
        _ => Err(config.error_at(
            seconds.position,
            stall_limit_problem(&seconds.value.to_string()),
        )),
    }
}

fn stall_limit_problem(found: &str) -> String {
    format!("expected a whole number of seconds from 1 to 3600, found {found}")
}

/// Says what the stall limit takes when its value is no whole number at all, such as
/// `2.5`, in the words of IMPORT-022.
pub(super) fn explain_stall_limit(
    parser: &dyn YamlParser,
    text: &str,
    error: ConfigError,
) -> ConfigError {
    if !matches!(
        error.problem,
        Problem::WrongKind { .. } | Problem::NumberTooLarge(_)
    ) {
        return error;
    }
    match stall_limit_value(parser, text) {
        Some((position, found)) if position == error.position => ConfigError {
            problem: Problem::Invalid(stall_limit_problem(&found)),
            ..error
        },
        _ => error,
    }
}

/// Where the value of the top-level stall limit starts, and its text if it is a
/// scalar.
fn stall_limit_value(parser: &dyn YamlParser, text: &str) -> Option<(Position, String)> {
    let Ok(Some(YamlNode {
        value: YamlValue::Mapping(entries),
        ..
    })) = parser.parse(text)
    else {
        return None;
    };
    let entry = entries.into_iter().find(|entry| {
        matches!(&entry.key.value, YamlValue::Scalar { text, .. } if text == STALL_LIMIT_KEY.name())
    })?;
    match entry.value.value {
        YamlValue::Scalar { text, .. } => Some((entry.value.position, text)),
        _ => None,
    }
}
