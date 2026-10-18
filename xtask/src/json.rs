// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Minimal JSON reader for the output of `cargo metadata`.
//!
//! Own code instead of a library (ADR 0009): the input is machine-generated, well
//! formed and trusted, and we only need to read it.

use std::collections::BTreeMap;

/// A parsed JSON value.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Json {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<Json>),
    Object(BTreeMap<String, Json>),
}

impl Json {
    /// Returns the member `key` of an object, or `Null` if it is missing.
    pub(crate) fn get(&self, key: &str) -> &Json {
        match self {
            Json::Object(members) => members.get(key).unwrap_or(&Json::Null),
            _ => &Json::Null,
        }
    }

    pub(crate) fn as_str(&self) -> Option<&str> {
        match self {
            Json::String(text) => Some(text),
            _ => None,
        }
    }

    pub(crate) fn as_bool(&self) -> Option<bool> {
        match self {
            Json::Bool(value) => Some(*value),
            _ => None,
        }
    }

    /// Returns the elements of an array; anything else counts as empty.
    pub(crate) fn items(&self) -> &[Json] {
        match self {
            Json::Array(items) => items,
            _ => &[],
        }
    }
}

/// Parses a complete JSON document.
pub(crate) fn parse(text: &str) -> Result<Json, String> {
    let mut parser = Parser {
        bytes: text.as_bytes(),
        position: 0,
    };
    let value = parser.value()?;
    parser.skip_whitespace();
    if parser.position != parser.bytes.len() {
        return Err(parser.error("trailing characters"));
    }
    Ok(value)
}

struct Parser<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl Parser<'_> {
    fn value(&mut self) -> Result<Json, String> {
        self.skip_whitespace();
        match self.peek() {
            Some(b'{') => self.object(),
            Some(b'[') => self.array(),
            Some(b'"') => self.string().map(Json::String),
            Some(b't') => self.literal("true", Json::Bool(true)),
            Some(b'f') => self.literal("false", Json::Bool(false)),
            Some(b'n') => self.literal("null", Json::Null),
            Some(b'-' | b'0'..=b'9') => self.number(),
            _ => Err(self.error("expected a value")),
        }
    }

    fn object(&mut self) -> Result<Json, String> {
        self.position += 1;
        let mut members = BTreeMap::new();
        self.skip_whitespace();
        if self.peek() == Some(b'}') {
            self.position += 1;
            return Ok(Json::Object(members));
        }
        loop {
            self.skip_whitespace();
            let key = self.string()?;
            self.skip_whitespace();
            self.expect(b':')?;
            let value = self.value()?;
            members.insert(key, value);
            self.skip_whitespace();
            match self.next() {
                Some(b',') => {}
                Some(b'}') => return Ok(Json::Object(members)),
                _ => return Err(self.error("expected , or }")),
            }
        }
    }

    fn array(&mut self) -> Result<Json, String> {
        self.position += 1;
        let mut items = Vec::new();
        self.skip_whitespace();
        if self.peek() == Some(b']') {
            self.position += 1;
            return Ok(Json::Array(items));
        }
        loop {
            items.push(self.value()?);
            self.skip_whitespace();
            match self.next() {
                Some(b',') => {}
                Some(b']') => return Ok(Json::Array(items)),
                _ => return Err(self.error("expected , or ]")),
            }
        }
    }

    fn string(&mut self) -> Result<String, String> {
        self.expect(b'"')?;
        let mut text = String::new();
        loop {
            let start = self.position;
            while let Some(byte) = self.peek() {
                if byte == b'"' || byte == b'\\' {
                    break;
                }
                self.position += 1;
            }
            let chunk = std::str::from_utf8(&self.bytes[start..self.position])
                .map_err(|_| self.error("invalid UTF-8"))?;
            text.push_str(chunk);
            match self.next() {
                Some(b'"') => return Ok(text),
                Some(b'\\') => text.push(self.escape()?),
                _ => return Err(self.error("unterminated string")),
            }
        }
    }

    fn escape(&mut self) -> Result<char, String> {
        let escaped = match self.next() {
            Some(b'"') => '"',
            Some(b'\\') => '\\',
            Some(b'/') => '/',
            Some(b'b') => '\u{8}',
            Some(b'f') => '\u{c}',
            Some(b'n') => '\n',
            Some(b'r') => '\r',
            Some(b't') => '\t',
            Some(b'u') => return self.unicode_escape(),
            _ => return Err(self.error("invalid escape")),
        };
        Ok(escaped)
    }

    fn unicode_escape(&mut self) -> Result<char, String> {
        let high = self.hex4()?;
        if !(0xD800..0xDC00).contains(&high) {
            return char::from_u32(high).ok_or_else(|| self.error("invalid code point"));
        }
        if self.next() != Some(b'\\') || self.next() != Some(b'u') {
            return Err(self.error("unpaired surrogate"));
        }
        let low = self.hex4()?;
        let code = 0x10000 + ((high - 0xD800) << 10) + (low.wrapping_sub(0xDC00) & 0x3FF);
        char::from_u32(code).ok_or_else(|| self.error("invalid surrogate pair"))
    }

    fn hex4(&mut self) -> Result<u32, String> {
        let end = self.position + 4;
        let digits = self
            .bytes
            .get(self.position..end)
            .and_then(|slice| std::str::from_utf8(slice).ok())
            .ok_or_else(|| self.error("short unicode escape"))?;
        let value = u32::from_str_radix(digits, 16).map_err(|_| self.error("bad hex digits"))?;
        self.position = end;
        Ok(value)
    }

    fn number(&mut self) -> Result<Json, String> {
        let start = self.position;
        while let Some(byte) = self.peek() {
            if !matches!(byte, b'-' | b'+' | b'.' | b'e' | b'E' | b'0'..=b'9') {
                break;
            }
            self.position += 1;
        }
        std::str::from_utf8(&self.bytes[start..self.position])
            .ok()
            .and_then(|digits| digits.parse().ok())
            .map(Json::Number)
            .ok_or_else(|| self.error("invalid number"))
    }

    fn literal(&mut self, word: &str, value: Json) -> Result<Json, String> {
        if self.bytes[self.position..].starts_with(word.as_bytes()) {
            self.position += word.len();
            Ok(value)
        } else {
            Err(self.error("invalid literal"))
        }
    }

    fn expect(&mut self, byte: u8) -> Result<(), String> {
        if self.next() == Some(byte) {
            Ok(())
        } else {
            Err(self.error(&format!("expected {:?}", char::from(byte))))
        }
    }

    fn skip_whitespace(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\n' | b'\r' | b'\t')) {
            self.position += 1;
        }
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.position).copied()
    }

    fn next(&mut self) -> Option<u8> {
        let byte = self.peek();
        self.position += 1;
        byte
    }

    fn error(&self, message: &str) -> String {
        format!("JSON: {message} at byte {}", self.position)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_nested_document() {
        let json = parse(r#"{"a": [1, true, null, "x"], "b": {"c": -2.5e1}}"#).unwrap();
        assert_eq!(json.get("a").items().len(), 4);
        assert_eq!(json.get("a").items()[1].as_bool(), Some(true));
        assert_eq!(json.get("b").get("c"), &Json::Number(-25.0));
        assert_eq!(json.get("missing"), &Json::Null);
    }

    #[test]
    fn decodes_escapes_and_surrogate_pairs() {
        let json = parse(r#""a\"b\\c\né😀""#).unwrap();
        assert_eq!(json.as_str(), Some("a\"b\\c\né😀"));
    }

    #[test]
    fn keeps_utf8_text() {
        assert_eq!(
            parse(r#""Bibliothèque""#).unwrap().as_str(),
            Some("Bibliothèque")
        );
    }

    #[test]
    fn rejects_malformed_input() {
        for text in ["{", "[1,]", r#"{"a" 1}"#, "tru", r#""abc"#, "1 2"] {
            assert!(parse(text).is_err(), "{text} should be rejected");
        }
    }
}
