// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

use std::fmt;

use thiserror::Error;

/// Stable, unique identifier of a module, e.g. `import` (architecture §4.2).
///
/// The id names the module's settings section, its storage namespace (table prefix
/// `<id>_`, architecture §4.4) and its log target. Because it is used as an SQL
/// identifier prefix, it is restricted to lowercase ASCII letters, digits and single
/// underscores, and it starts with a letter.
///
/// ```
/// use kernel::ModuleId;
///
/// let id = ModuleId::new("import")?;
/// assert_eq!(id.as_str(), "import");
/// # Ok::<(), kernel::ModuleIdError>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ModuleId(String);

/// Error returned when a string is not a valid [`ModuleId`].
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ModuleIdError {
    /// The id is empty.
    #[error("a module id must not be empty")]
    Empty,
    /// The id is longer than [`ModuleId::MAX_LEN`] characters.
    #[error("module id {id:?} is longer than {max} characters", max = ModuleId::MAX_LEN)]
    TooLong {
        /// The rejected id.
        id: String,
    },
    /// The id does not start with a lowercase ASCII letter.
    #[error("module id {id:?} must start with a lowercase letter")]
    InvalidStart {
        /// The rejected id.
        id: String,
    },
    /// The id contains a character other than `a-z`, `0-9` and `_`.
    #[error("module id {id:?} contains {character:?}; allowed are a-z, 0-9 and _")]
    InvalidCharacter {
        /// The rejected id.
        id: String,
        /// The first character that is not allowed.
        character: char,
    },
    /// The id ends with an underscore or contains two underscores in a row.
    #[error("module id {id:?} must not end with _ or contain __")]
    InvalidUnderscore {
        /// The rejected id.
        id: String,
    },
}

impl ModuleId {
    /// Maximum length of a module id in characters.
    pub const MAX_LEN: usize = 32;

    /// Creates a module id after validating it.
    ///
    /// # Errors
    ///
    /// Returns a [`ModuleIdError`] describing the first rule the id violates.
    pub fn new(id: &str) -> Result<Self, ModuleIdError> {
        validate(id)?;
        Ok(ModuleId(id.to_owned()))
    }

    /// Returns the id as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ModuleId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

fn validate(id: &str) -> Result<(), ModuleIdError> {
    let Some(first) = id.chars().next() else {
        return Err(ModuleIdError::Empty);
    };
    if id.len() > ModuleId::MAX_LEN {
        return Err(ModuleIdError::TooLong { id: id.to_owned() });
    }
    if !first.is_ascii_lowercase() {
        return Err(ModuleIdError::InvalidStart { id: id.to_owned() });
    }
    if let Some(character) = id
        .chars()
        .find(|c| !(c.is_ascii_lowercase() || c.is_ascii_digit() || *c == '_'))
    {
        return Err(ModuleIdError::InvalidCharacter {
            id: id.to_owned(),
            character,
        });
    }
    if id.ends_with('_') || id.contains("__") {
        return Err(ModuleIdError::InvalidUnderscore { id: id.to_owned() });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_the_planned_module_ids() {
        for id in [
            "import",
            "library",
            "map",
            "people",
            "immich",
            "migrate_digikam",
        ] {
            assert_eq!(ModuleId::new(id).unwrap().as_str(), id);
        }
    }

    #[test]
    fn rejects_empty_id() {
        assert_eq!(ModuleId::new(""), Err(ModuleIdError::Empty));
    }

    #[test]
    fn rejects_id_longer_than_the_limit() {
        let id = "a".repeat(ModuleId::MAX_LEN + 1);
        assert_eq!(ModuleId::new(&id), Err(ModuleIdError::TooLong { id }));
        assert!(ModuleId::new(&"a".repeat(ModuleId::MAX_LEN)).is_ok());
    }

    #[test]
    fn rejects_id_not_starting_with_a_letter() {
        for id in ["1import", "_import", "Import"] {
            assert_eq!(
                ModuleId::new(id),
                Err(ModuleIdError::InvalidStart { id: id.to_owned() })
            );
        }
    }

    #[test]
    fn rejects_hyphens_and_non_ascii() {
        assert_eq!(
            ModuleId::new("migrate-digikam"),
            Err(ModuleIdError::InvalidCharacter {
                id: "migrate-digikam".to_owned(),
                character: '-',
            })
        );
        assert_eq!(
            ModuleId::new("bibliothèque"),
            Err(ModuleIdError::InvalidCharacter {
                id: "bibliothèque".to_owned(),
                character: 'è',
            })
        );
    }

    #[test]
    fn rejects_trailing_and_double_underscores() {
        for id in ["import_", "migrate__digikam"] {
            assert_eq!(
                ModuleId::new(id),
                Err(ModuleIdError::InvalidUnderscore { id: id.to_owned() })
            );
        }
    }

    #[test]
    fn displays_as_the_plain_id() {
        assert_eq!(ModuleId::new("map").unwrap().to_string(), "map");
    }
}
