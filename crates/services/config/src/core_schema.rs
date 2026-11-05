// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! How the YAML 1.2 core schema reads a plain scalar (YAML 1.2.2, §10.3.2).

/// What a plain scalar is under the core schema.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CoreType {
    Null,
    Boolean(bool),
    Integer,
    Float,
    Text,
}

/// Resolves the text of a plain scalar.
pub(crate) fn resolve(text: &str) -> CoreType {
    match text {
        "" | "~" | "null" | "Null" | "NULL" => CoreType::Null,
        "true" | "True" | "TRUE" => CoreType::Boolean(true),
        "false" | "False" | "FALSE" => CoreType::Boolean(false),
        _ if is_integer(text) => CoreType::Integer,
        _ if is_float(text) => CoreType::Float,
        _ => CoreType::Text,
    }
}

/// The value of a plain scalar that [`resolve`] reads as an integer, or `None` when
/// it does not fit an `i64`.
pub(crate) fn integer_value(text: &str) -> Option<i64> {
    if let Some(octal) = text.strip_prefix("0o") {
        i64::from_str_radix(octal, 8).ok()
    } else if let Some(hexadecimal) = text.strip_prefix("0x") {
        i64::from_str_radix(hexadecimal, 16).ok()
    } else {
        text.parse().ok()
    }
}

/// Whether `text` starts with a date `YYYY-MM-DD`, which YAML 1.1 tools read as a
/// timestamp, so text that does must be quoted (ADR 0011, rule 6).
pub(crate) fn starts_with_date(text: &str) -> bool {
    let bytes = text.as_bytes();
    let digits =
        |range: std::ops::Range<usize>| range.into_iter().all(|i| bytes[i].is_ascii_digit());
    bytes.len() >= 10
        && digits(0..4)
        && bytes[4] == b'-'
        && digits(5..7)
        && bytes[7] == b'-'
        && digits(8..10)
}

fn is_integer(text: &str) -> bool {
    if let Some(octal) = text.strip_prefix("0o") {
        return !octal.is_empty() && octal.bytes().all(|b| (b'0'..=b'7').contains(&b));
    }
    if let Some(hexadecimal) = text.strip_prefix("0x") {
        return !hexadecimal.is_empty() && hexadecimal.bytes().all(|b| b.is_ascii_hexdigit());
    }
    let digits = without_sign(text);
    !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit())
}

fn is_float(text: &str) -> bool {
    let unsigned = without_sign(text);
    if matches!(unsigned, ".inf" | ".Inf" | ".INF") || matches!(text, ".nan" | ".NaN" | ".NAN") {
        return true;
    }
    let (mantissa, exponent) = match unsigned.find(['e', 'E']) {
        Some(index) => (&unsigned[..index], Some(&unsigned[index + 1..])),
        None => (unsigned, None),
    };
    let (whole, fraction) = match mantissa.split_once('.') {
        Some((whole, fraction)) => (whole, Some(fraction)),
        None => (mantissa, None),
    };
    let all_digits = |part: &str| part.bytes().all(|b| b.is_ascii_digit());
    let mantissa_valid = all_digits(whole)
        && fraction.is_none_or(all_digits)
        && (!whole.is_empty() || fraction.is_some_and(|f| !f.is_empty()));
    let exponent_valid = exponent.is_none_or(|e| {
        let digits = without_sign(e);
        !digits.is_empty() && all_digits(digits)
    });
    mantissa_valid && exponent_valid
}

fn without_sign(text: &str) -> &str {
    text.strip_prefix(['-', '+']).unwrap_or(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_the_core_schema() {
        for (text, expected) in [
            ("", CoreType::Null),
            ("~", CoreType::Null),
            ("NULL", CoreType::Null),
            ("True", CoreType::Boolean(true)),
            ("FALSE", CoreType::Boolean(false)),
            ("no", CoreType::Text),
            ("yes", CoreType::Text),
            ("off", CoreType::Text),
            ("2026", CoreType::Integer),
            ("-7", CoreType::Integer),
            ("+7", CoreType::Integer),
            ("0o17", CoreType::Integer),
            ("0x1F", CoreType::Integer),
            ("0x", CoreType::Text),
            ("1.5", CoreType::Float),
            ("1.", CoreType::Float),
            (".5", CoreType::Float),
            ("1e3", CoreType::Float),
            ("-.inf", CoreType::Float),
            (".NaN", CoreType::Float),
            (".", CoreType::Text),
            ("1e", CoreType::Text),
            ("/a", CoreType::Text),
            ("1_000", CoreType::Text),
        ] {
            assert_eq!(resolve(text), expected, "{text:?}");
        }
    }

    #[test]
    fn reads_integer_values() {
        assert_eq!(integer_value("0o17"), Some(15));
        assert_eq!(integer_value("0x1F"), Some(31));
        assert_eq!(integer_value("-12"), Some(-12));
        assert_eq!(integer_value("+12"), Some(12));
        assert_eq!(integer_value("99999999999999999999"), None);
    }

    #[test]
    fn recognizes_a_leading_date() {
        assert!(starts_with_date("2026-11-05"));
        assert!(starts_with_date("2026-11-05 Smith"));
        assert!(!starts_with_date("2026-11-5"));
        assert!(!starts_with_date("Smith 2026-11-05"));
        assert!(!starts_with_date("Müller"));
    }
}
