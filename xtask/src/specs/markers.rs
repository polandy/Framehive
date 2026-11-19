// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! `/// spec: <ID>.<n>` markers that link tests to acceptance criteria.

pub(crate) const TEST_MARKER: &str = "/// spec:";

/// A `/// spec: <ID>.<n>` reference in a source file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Marker {
    pub(crate) location: String,
    pub(crate) spec_id: String,
    pub(crate) number: u32,
}

impl Marker {
    pub(super) fn criterion(&self) -> String {
        format!("{}.{}", self.spec_id, self.number)
    }
}

/// Finds test markers; a marker line may list several criteria separated by commas.
pub(crate) fn test_markers(file: &str, source: &str) -> Vec<Marker> {
    let mut markers = Vec::new();
    for (index, line) in source.lines().enumerate() {
        let Some(list) = line.trim_start().strip_prefix(TEST_MARKER) else {
            continue;
        };
        for criterion in list.split(',').map(str::trim) {
            let parsed = criterion
                .rsplit_once('.')
                .and_then(|(id, number)| Some((id, number.parse::<u32>().ok()?)));
            let (spec_id, number) = parsed.unwrap_or((criterion, 0));
            markers.push(Marker {
                location: format!("{file}:{}", index + 1),
                spec_id: spec_id.to_owned(),
                number,
            });
        }
    }
    markers
}

/// The locations of markers that do not sit on a test function, that is in the run of
/// attributes and comments right above a `fn` with a `#[test]` attribute.
pub(crate) fn unbound_markers(file: &str, source: &str) -> Vec<String> {
    let lines: Vec<&str> = source.lines().collect();
    lines
        .iter()
        .enumerate()
        .filter(|(_, line)| line.trim_start().starts_with(TEST_MARKER))
        .filter(|(index, _)| !on_test_function(&lines, *index))
        .map(|(index, _)| format!("{file}:{}", index + 1))
        .collect()
}

fn on_test_function(lines: &[&str], marker: usize) -> bool {
    let above = lines[..marker]
        .iter()
        .rev()
        .take_while(|line| is_decoration(line));
    let mut below = lines[marker + 1..].iter();
    let mut decorations: Vec<&str> = above.copied().collect();
    let item = loop {
        match below.next() {
            Some(line) if is_decoration(line) => decorations.push(line),
            other => break other.map_or("", |line| line.trim_start()),
        }
    };
    let is_function = item.starts_with("fn ") || item.starts_with("async fn ");
    is_function && decorations.iter().any(|line| is_test_attribute(line))
}

/// An attribute or a comment line, as they stand between a doc comment and its item.
fn is_decoration(line: &str) -> bool {
    let line = line.trim_start();
    line.starts_with("#[") || line.starts_with("//")
}

/// `#[test]`, or a test attribute of a crate such as `#[tokio::test]`.
fn is_test_attribute(line: &str) -> bool {
    let line = line.trim();
    line == "#[test]" || line.ends_with("::test]")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_markers_with_several_criteria() {
        let source = "#[test]\n    /// spec: CORE-009.1, CORE-009.2\nfn test() {}\n";
        assert_eq!(
            test_markers("a.rs", source),
            [
                Marker {
                    location: "a.rs:2".to_owned(),
                    spec_id: "CORE-009".to_owned(),
                    number: 1
                },
                Marker {
                    location: "a.rs:2".to_owned(),
                    spec_id: "CORE-009".to_owned(),
                    number: 2
                },
            ]
        );
    }

    #[test]
    fn a_marker_above_or_below_the_test_attribute_is_bound() {
        let source = "/// spec: CORE-009.1\n#[test]\nfn a() {}\n\
                      #[test]\n#[cfg(unix)]\n/// spec: CORE-009.2\n// why\nfn b() {}\n";
        assert_eq!(unbound_markers("a.rs", source), Vec::<String>::new());
    }

    #[test]
    fn a_marker_on_a_function_without_a_test_attribute_is_unbound() {
        let source = "/// spec: CORE-009.1\nfn helper() {}\n";
        assert_eq!(unbound_markers("a.rs", source), ["a.rs:1"]);
    }

    #[test]
    fn a_marker_on_something_other_than_a_function_is_unbound() {
        let source = "#[test]\n/// spec: CORE-009.1\nconst A: u8 = 1;\n\n/// spec: CORE-009.2\n";
        assert_eq!(unbound_markers("a.rs", source), ["a.rs:2", "a.rs:5"]);
    }

    #[test]
    fn a_crate_test_attribute_binds_a_marker() {
        let source = "/// spec: CORE-009.1\n#[tokio::test]\nasync fn a() {}\n";
        assert_eq!(unbound_markers("a.rs", source), Vec::<String>::new());
    }
}
