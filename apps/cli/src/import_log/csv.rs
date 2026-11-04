// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The log of one run as RFC 4180 CSV (CLI-009). Own code: the format is small and
//! fixed, so it needs no library (ADR 0009).

use history::{CopyRecord, RunLog};

use super::{capture_time_text, shown};

/// The header row.
const HEADER: [&str; 9] = [
    "run",
    "destination",
    "source_path",
    "original_file_name",
    "size",
    "capture_time",
    "checksum",
    "target",
    "recorded_at",
];

/// The header row and one row per copy, each ending in CRLF.
pub(super) fn run_log(log: &RunLog) -> String {
    let mut rows = vec![row(&HEADER.map(str::to_owned))];
    let run = log.run.id.number().to_string();
    rows.extend(log.copies.iter().map(|copy| row(&fields(&run, copy))));
    rows.concat()
}

fn fields(run: &str, copy: &CopyRecord) -> [String; 9] {
    [
        run.to_owned(),
        shown(&copy.destination),
        shown(&copy.source_path),
        copy.original_file_name.clone(),
        copy.size.to_string(),
        copy.capture_time.map(capture_time_text).unwrap_or_default(),
        copy.checksum.to_string(),
        copy.target.clone(),
        copy.recorded_at.to_string(),
    ]
}

fn row(fields: &[String]) -> String {
    let quoted: Vec<String> = fields.iter().map(|value| field(value)).collect();
    format!("{}\r\n", quoted.join(","))
}

/// `value` as a CSV field: quoted when it holds a comma, a double quote or a line
/// break, with each double quote doubled.
fn field(value: &str) -> String {
    if value.contains([',', '"', '\r', '\n']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::import_log::fixtures::{copy, log};

    #[test]
    fn a_field_with_a_comma_quote_or_line_break_is_quoted() {
        assert_eq!(field("plain"), "plain");
        assert_eq!(field("a,\"b\".jpg"), "\"a,\"\"b\"\".jpg\"");
        assert_eq!(field("line\nbreak"), "\"line\nbreak\"");
        assert_eq!(field("carriage\rreturn"), "\"carriage\rreturn\"");
    }

    #[test]
    fn the_log_is_a_header_and_one_row_per_copy() {
        let csv = run_log(&log());

        assert_eq!(
            csv,
            "run,destination,source_path,original_file_name,size,capture_time,checksum,target,recorded_at\r\n\
             3,/library,\"DCIM/a,\"\"b\"\".jpg\",\"a,\"\"b\"\".jpg\",10,2026-10-17T14:05:09,blake3:0101,\"2026/a,\"\"b\"\".jpg\",2026-10-31T11:00:00+02:00\r\n\
             3,/backup,\"DCIM/a,\"\"b\"\".jpg\",\"a,\"\"b\"\".jpg\",10,2026-10-17T14:05:09,blake3:0101,\"2026/a,\"\"b\"\".jpg\",2026-10-31T11:00:00+02:00\r\n"
        );
    }

    #[test]
    fn an_absent_capture_time_is_an_empty_field() {
        let csv = run_log(&RunLog {
            copies: vec![CopyRecord {
                capture_time: None,
                ..copy("/library")
            }],
            ..log()
        });

        assert!(csv.contains(",10,,blake3:0101,"), "{csv}");
    }
}
