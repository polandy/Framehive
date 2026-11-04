// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The text of the counter file (ADR 0029, section 2).

use naming::{CounterDate, StoredCounters};

const FORMAT_VERSION_LINE: &str = "format_version=1";

/// The largest value of a `*_last` key (ADR 0029, section 2).
const MAX_LAST: u32 = 999_999;

/// Why the text is broken: the line, counted from 1, and what is wrong with it.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Broken {
    pub(crate) line: usize,
    pub(crate) reason: String,
}

/// Parses the text of a counter file. Missing keys are 0.
pub(crate) fn parse(text: &str) -> Result<StoredCounters, Broken> {
    let mut lines = text
        .lines()
        .enumerate()
        .map(|(index, line)| (index + 1, line))
        .filter(|(_, line)| !line.trim().is_empty() && !line.starts_with('#'));
    match lines.next() {
        Some((_, FORMAT_VERSION_LINE)) => {}
        Some((line, text)) => {
            return Err(Broken {
                line,
                reason: format!("{text:?} is not {FORMAT_VERSION_LINE}"),
            });
        }
        None => {
            return Err(Broken {
                line: 1,
                reason: format!("the file has no {FORMAT_VERSION_LINE}"),
            });
        }
    }
    let mut values = Values::default();
    for (line, text) in lines {
        values.add(text).map_err(|reason| Broken { line, reason })?;
    }
    Ok(values.into_counters())
}

/// The values read so far; `None` for a key not seen yet.
#[derive(Default)]
struct Values {
    imported_today_date: Option<CounterDate>,
    imported_today_last: Option<u32>,
    stored_last: Option<u32>,
}

impl Values {
    fn add(&mut self, text: &str) -> Result<(), String> {
        let (key, value) = text
            .split_once('=')
            .ok_or_else(|| format!("{text:?} is not understood"))?;
        let repeated = || format!("{key} appears twice");
        match key {
            "imported_today_date" => {
                let date = CounterDate::parse(value)
                    .ok_or_else(|| format!("{value:?} is not a date YYYY-MM-DD"))?;
                set_once(&mut self.imported_today_date, date).ok_or_else(repeated)
            }
            "imported_today_last" => {
                set_once(&mut self.imported_today_last, last(value)?).ok_or_else(repeated)
            }
            "stored_last" => set_once(&mut self.stored_last, last(value)?).ok_or_else(repeated),
            _ => Err(format!("{text:?} is not understood")),
        }
    }

    fn into_counters(self) -> StoredCounters {
        StoredCounters {
            imported_today_date: self.imported_today_date,
            imported_today_last: self.imported_today_last.unwrap_or(0),
            stored_last: self.stored_last.unwrap_or(0),
        }
    }
}

/// Stores `value` in an empty `slot`; `None` when the slot is already set.
fn set_once<T>(slot: &mut Option<T>, value: T) -> Option<()> {
    if slot.is_some() {
        return None;
    }
    *slot = Some(value);
    Some(())
}

/// A `*_last` value: decimal digits, 0 to 999999.
fn last(value: &str) -> Result<u32, String> {
    let digits = !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit());
    value
        .parse::<u32>()
        .ok()
        .filter(|&number| digits && number <= MAX_LAST)
        .ok_or_else(|| format!("{value:?} is not a number from 0 to {MAX_LAST}"))
}

/// The text of a counter file with `counters`.
pub(crate) fn format(counters: &StoredCounters) -> String {
    let date = counters
        .imported_today_date
        .map(|date| format!("imported_today_date={date}\n"))
        .unwrap_or_default();
    format!(
        "# Framehive keeps its sequence counters here (IMPORT-002).\n{FORMAT_VERSION_LINE}\n\
         {date}imported_today_last={}\nstored_last={}\n",
        counters.imported_today_last, counters.stored_last
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn broken(text: &str) -> Broken {
        parse(text).unwrap_err()
    }

    #[test]
    fn written_text_reads_back() {
        let counters = StoredCounters {
            imported_today_date: CounterDate::parse("2026-10-17"),
            imported_today_last: 15,
            stored_last: 250,
        };

        assert_eq!(parse(&format(&counters)), Ok(counters));
    }

    #[test]
    fn missing_keys_are_zero_and_comments_are_ignored() {
        let text = "# note\n\nformat_version=1\n# another\nstored_last=7\n";

        assert_eq!(
            parse(text),
            Ok(StoredCounters {
                stored_last: 7,
                ..StoredCounters::default()
            })
        );
    }

    #[test]
    fn broken_lines_are_named() {
        let version = "format_version=1\n";

        assert_eq!(broken("stored_last=1\n").line, 1);
        assert_eq!(broken("").line, 1);
        assert_eq!(broken("format_version=2\n").line, 1);
        assert_eq!(broken(&format!("{version}stored_last=many\n")).line, 2);
        assert_eq!(broken(&format!("{version}stored_last=1000000\n")).line, 2);
        assert_eq!(broken(&format!("{version}stored_last=+5\n")).line, 2);
        assert_eq!(
            broken(&format!("{version}\nstored_last=1\nstored_last=2\n")).line,
            4
        );
        assert_eq!(
            broken(&format!("{version}imported_today_date=17.10.2026\n")).line,
            2
        );
        assert_eq!(broken(&format!("{version}session_last=3\n")).line, 2);
        assert_eq!(broken(&format!("{version}no equals sign\n")).line, 2);
    }
}
