// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! `/// spec: <ID>.<n>` markers that link tests to acceptance criteria.

const TEST_MARKER: &str = "/// spec:";

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
}
