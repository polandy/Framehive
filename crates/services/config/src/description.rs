// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The description of a file type's keys (ADR 0032, decision 2), from which the
//! strict reading and the JSON Schema follow.
//!
//! Descriptions are `const`, so a file type is written down once, as a constant:
//!
//! ```
//! use config::{DefaultValue, FileType, Key};
//!
//! const NAMING: &[Key] = &[
//!     Key::text("on_conflict", "Conflict policy")
//!         .with_default(DefaultValue::Text("add_suffix")),
//! ];
//! const PROFILE: FileType = FileType::new(
//!     "import-profile",
//!     1,
//!     &[
//!         Key::text("destination", "Folder to import into"),
//!         Key::mapping("naming", "How imported files are named", NAMING),
//!     ],
//! );
//! assert_eq!(PROFILE.keys().len(), 2);
//! ```

/// One kind of configuration file, such as the import profile.
///
/// Every file type has the key `schema_version`, which is not part of
/// [`FileType::keys`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileType {
    name: &'static str,
    highest_version: u32,
    keys: &'static [Key],
}

impl FileType {
    /// A file type `name` (such as `import-profile`) whose highest `schema_version`
    /// is `highest_version`, with the top-level `keys` besides `schema_version`.
    #[must_use]
    pub const fn new(name: &'static str, highest_version: u32, keys: &'static [Key]) -> Self {
        FileType {
            name,
            highest_version,
            keys,
        }
    }

    /// The name of the file type.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        self.name
    }

    /// The highest `schema_version` this Framehive reads.
    #[must_use]
    pub const fn highest_version(&self) -> u32 {
        self.highest_version
    }

    /// The top-level keys besides `schema_version`, in the order of the description.
    #[must_use]
    pub const fn keys(&self) -> &'static [Key] {
        self.keys
    }
}

/// One key of a file type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Key {
    name: &'static str,
    kind: KeyKind,
    required: bool,
    default: Option<DefaultValue>,
    description: &'static str,
}

/// What a key holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyKind {
    /// Text.
    Text,
    /// A whole number.
    WholeNumber,
    /// `true` or `false`.
    Boolean,
    /// A list of text, such as job codes or paths.
    ListOfText,
    /// Text that must be one of these texts, such as a conflict policy.
    Choice(&'static [&'static str]),
    /// A nested mapping with these keys.
    Mapping(&'static [Key]),
}

/// The documented default of an optional key, published in the JSON Schema.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DefaultValue {
    /// Text.
    Text(&'static str),
    /// A whole number.
    WholeNumber(i64),
    /// `true` or `false`.
    Boolean(bool),
}

impl Key {
    const fn new(name: &'static str, kind: KeyKind, description: &'static str) -> Self {
        Key {
            name,
            kind,
            required: false,
            default: None,
            description,
        }
    }

    /// An optional text key with a one-line `description`.
    #[must_use]
    pub const fn text(name: &'static str, description: &'static str) -> Self {
        Self::new(name, KeyKind::Text, description)
    }

    /// An optional whole-number key.
    #[must_use]
    pub const fn whole_number(name: &'static str, description: &'static str) -> Self {
        Self::new(name, KeyKind::WholeNumber, description)
    }

    /// An optional `true` or `false` key.
    #[must_use]
    pub const fn boolean(name: &'static str, description: &'static str) -> Self {
        Self::new(name, KeyKind::Boolean, description)
    }

    /// An optional list of text.
    #[must_use]
    pub const fn list_of_text(name: &'static str, description: &'static str) -> Self {
        Self::new(name, KeyKind::ListOfText, description)
    }

    /// An optional text key whose value must be one of `choices`, listed in this order
    /// in messages and in the JSON Schema.
    #[must_use]
    pub const fn choice(
        name: &'static str,
        description: &'static str,
        choices: &'static [&'static str],
    ) -> Self {
        Self::new(name, KeyKind::Choice(choices), description)
    }

    /// An optional nested mapping with `keys`.
    #[must_use]
    pub const fn mapping(
        name: &'static str,
        description: &'static str,
        keys: &'static [Key],
    ) -> Self {
        Self::new(name, KeyKind::Mapping(keys), description)
    }

    /// The same key, required in its mapping.
    #[must_use]
    pub const fn required(mut self) -> Self {
        self.required = true;
        self
    }

    /// The same key, with a documented default.
    #[must_use]
    pub const fn with_default(mut self, default: DefaultValue) -> Self {
        self.default = Some(default);
        self
    }

    /// The key as written in a file.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        self.name
    }

    /// What the key holds.
    #[must_use]
    pub const fn kind(&self) -> KeyKind {
        self.kind
    }

    /// Whether the key must be present in its mapping.
    #[must_use]
    pub const fn is_required(&self) -> bool {
        self.required
    }

    /// The documented default, if any.
    #[must_use]
    pub const fn default(&self) -> Option<DefaultValue> {
        self.default
    }

    /// The one-line description.
    #[must_use]
    pub const fn description(&self) -> &'static str {
        self.description
    }
}
