// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The log of one run as one JSON object (CLI-009, RFC 8259). Own code: the format is
//! small and fixed, so it needs no library (ADR 0009).

use std::fmt::Write;

use history::{CopyRecord, RunLog};

use super::shown;

/// The log as one JSON object on one line, followed by a newline.
pub(super) fn run_log(log: &RunLog) -> String {
    let run = &log.run;
    let copies: Vec<String> = log.copies.iter().map(copy_object).collect();
    format!(
        "{{\"run\": {}, \"started_at\": {}, \"finished_at\": {}, \"source\": {}, \
         \"import_destination\": {}, \"video_destination\": {}, \"copies\": [{}]}}\n",
        run.id.number(),
        string(&run.start.started_at.to_string()),
        optional(run.finished_at.map(|time| time.to_string())),
        string(&shown(&run.start.source)),
        string(&shown(&run.start.import_destination)),
        optional(run.start.video_destination.as_deref().map(shown)),
        copies.join(", "),
    )
}

fn copy_object(copy: &CopyRecord) -> String {
    format!(
        "{{\"destination\": {}, \"source_path\": {}, \"original_file_name\": {}, \
         \"size\": {}, \"capture_time\": {}, \"checksum\": {}, \"target\": {}, \
         \"recorded_at\": {}}}",
        string(&shown(&copy.destination)),
        string(&shown(&copy.source_path)),
        string(&copy.original_file_name),
        copy.size,
        optional(copy.capture_time.map(|time| time.to_string())),
        string(&copy.checksum.to_string()),
        string(&copy.target),
        string(&copy.recorded_at.to_string()),
    )
}

/// A JSON string, or `null` for `None`.
fn optional(value: Option<String>) -> String {
    value.map_or_else(|| "null".to_owned(), |value| string(&value))
}

/// `value` as a JSON string: quotes, backslashes and control characters escaped.
fn string(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len() + 2);
    escaped.push('"');
    for character in value.chars() {
        match character {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            control if control < ' ' => {
                // Writing to a String does not fail.
                let _ = write!(escaped, "\\u{:04x}", u32::from(control));
            }
            other => escaped.push(other),
        }
    }
    escaped.push('"');
    escaped
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::import_log::fixtures::{copy, log, run};

    #[test]
    fn a_string_escapes_quotes_backslashes_and_control_characters() {
        assert_eq!(
            string("a\"b\\c\nd\re\tf\u{1}g\u{7f}h\u{fffd}"),
            "\"a\\\"b\\\\c\\nd\\re\\tf\\u0001g\u{7f}h\u{fffd}\""
        );
    }

    #[test]
    fn the_log_is_one_object_with_every_field() {
        let json = run_log(&RunLog {
            copies: vec![copy("/library")],
            ..log()
        });

        assert_eq!(
            json,
            "{\"run\": 3, \"started_at\": \"2026-10-31T11:00:00+02:00\", \
             \"finished_at\": \"2026-10-31T11:00:00+02:00\", \"source\": \"/media/card\", \
             \"import_destination\": \"/library\", \"video_destination\": null, \"copies\": [{\"destination\": \"/library\", \
             \"source_path\": \"DCIM/a,\\\"b\\\".jpg\", \"original_file_name\": \"a,\\\"b\\\".jpg\", \
             \"size\": 10, \"capture_time\": \"2026-10-17T14:05:09\", \"checksum\": \"blake3:0101\", \
             \"target\": \"2026/a,\\\"b\\\".jpg\", \"recorded_at\": \"2026-10-31T11:00:00+02:00\"}]}\n"
        );
    }

    #[test]
    fn absent_times_are_null() {
        let json = run_log(&RunLog {
            run: run(None),
            copies: vec![CopyRecord {
                capture_time: None,
                ..copy("/library")
            }],
        });

        assert!(json.contains("\"finished_at\": null"), "{json}");
        assert!(json.contains("\"capture_time\": null"), "{json}");
    }
}
