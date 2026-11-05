// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The "did you mean …?" of an unknown key (CORE-006, "Keys").

/// The most edits an unknown key may be away from a suggested one.
const MAX_DISTANCE: usize = 2;

/// The known key nearest to `unknown`, at most [`MAX_DISTANCE`] edits away; on a
/// tie, the first in `known`.
pub(crate) fn nearest<'a>(
    unknown: &str,
    known: impl IntoIterator<Item = &'a str>,
) -> Option<&'a str> {
    let mut best: Option<(usize, &str)> = None;
    for candidate in known {
        let distance = edit_distance(unknown, candidate);
        if distance <= MAX_DISTANCE && best.is_none_or(|(nearest, _)| distance < nearest) {
            best = Some((distance, candidate));
        }
    }
    best.map(|(_, candidate)| candidate)
}

/// The Levenshtein distance: single-character insertions, deletions and
/// substitutions, counted in characters.
fn edit_distance(from: &str, to: &str) -> usize {
    let to: Vec<char> = to.chars().collect();
    let mut previous: Vec<usize> = (0..=to.len()).collect();
    for (i, from_char) in from.chars().enumerate() {
        let mut current = vec![i + 1];
        for (j, to_char) in to.iter().enumerate() {
            let substitution = previous[j] + usize::from(from_char != *to_char);
            let deletion = previous[j + 1] + 1;
            let insertion = current[j] + 1;
            current.push(substitution.min(deletion).min(insertion));
        }
        previous = current;
    }
    previous[to.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_edits_in_characters() {
        assert_eq!(edit_distance("destinaton", "destination"), 1);
        assert_eq!(edit_distance("colour", "color"), 1);
        assert_eq!(edit_distance("", "abc"), 3);
        assert_eq!(edit_distance("Müller", "Muller"), 1);
        assert_eq!(edit_distance("kitten", "sitting"), 3);
    }

    #[test]
    fn suggests_the_nearest_and_the_first_on_a_tie() {
        assert_eq!(
            nearest("destinaton", ["schema_version", "destination"]),
            Some("destination")
        );
        assert_eq!(nearest("ab", ["ax", "ay"]), Some("ax"));
        assert_eq!(nearest("abc", ["xyz", "abd", "abc_"]), Some("abd"));
        assert_eq!(nearest("colour", ["destination"]), None);
    }
}
