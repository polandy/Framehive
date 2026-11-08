// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The values the import writes to each photo's sidecar, from the `metadata` keys of
//! `config.yaml` and of the import profile (IMPORT-019, ADR 0040 §2).

use std::fmt;

use config::{ConfigError, ConfigFile, DefaultValue, Key, Located, Values};
use metadata::xmp::SidecarNaming;

const ORIGINAL_WITH_EXTENSION: &str = "original_with_extension";
const ORIGINAL_WITHOUT_EXTENSION: &str = "original_without_extension";

/// The keys of the mapping `metadata` (IMPORT-019, "Values").
pub(crate) const METADATA_KEYS: &[Key] = &[
    Key::list_of_text("creator", "Creators of the photos, in this order (dc:creator)"),
    Key::text(
        "copyright",
        "Copyright notice (dc:rights); {year} is the year of the capture time",
    ),
    Key::list_of_text(
        "keywords",
        "Keywords; / separates the levels of a hierarchy, such as Holiday/Beach",
    ),
    Key::choice(
        "sidecar_naming",
        "Sidecar name: original_with_extension (DSCF0042.RAF.xmp) or original_without_extension (DSCF0042.xmp)",
        &[ORIGINAL_WITH_EXTENSION, ORIGINAL_WITHOUT_EXTENSION],
    )
    .with_default(DefaultValue::Text(ORIGINAL_WITH_EXTENSION)),
];

/// The `metadata` key with its description.
pub(crate) const METADATA_KEY: Key = Key::mapping(
    "metadata",
    "Metadata the import writes to each photo's XMP sidecar (IMPORT-019)",
    METADATA_KEYS,
);

/// The token in the copyright notice that becomes the year.
const YEAR_TOKEN: &str = "{year}";

/// One keyword: its levels from the top, each one not empty, as
/// `Holiday/Beach` gives `Holiday` › `Beach`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Keyword {
    levels: Vec<String>,
}

/// Why a text is not a keyword (IMPORT-019, "Values").
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum KeywordError {
    /// The keyword is empty.
    #[error("a keyword must not be empty")]
    Empty,
    /// The keyword starts or ends with `/`, or holds `//`.
    #[error("a keyword must not start or end with / or contain //")]
    EmptyLevel,
    /// The keyword holds `|`, which separates levels in `lr:hierarchicalSubject`.
    #[error("a keyword must not contain |")]
    Pipe,
}

impl Keyword {
    /// Reads a keyword; `/` separates its levels.
    ///
    /// ```
    /// use import::Keyword;
    ///
    /// let keyword = Keyword::parse("Holiday/Beach").unwrap();
    /// assert_eq!(keyword.levels(), ["Holiday", "Beach"]);
    /// assert!(Keyword::parse("Holiday//Beach").is_err());
    /// ```
    ///
    /// # Errors
    ///
    /// A [`KeywordError`] for an empty keyword, an empty level or a `|`.
    pub fn parse(text: &str) -> Result<Self, KeywordError> {
        if text.is_empty() {
            return Err(KeywordError::Empty);
        }
        if text.contains('|') {
            return Err(KeywordError::Pipe);
        }
        let levels: Vec<String> = text.split('/').map(str::to_owned).collect();
        if levels.iter().any(String::is_empty) {
            return Err(KeywordError::EmptyLevel);
        }
        Ok(Keyword { levels })
    }

    /// The levels, from the top of the hierarchy to the keyword itself.
    #[must_use]
    pub fn levels(&self) -> &[String] {
        &self.levels
    }
}

impl fmt::Display for Keyword {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.levels.join("/"))
    }
}

/// The `metadata` keys one file sets; a key the file leaves out is `None`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MetadataKeys {
    /// `metadata.creator`.
    pub creator: Option<Vec<String>>,
    /// `metadata.copyright`.
    pub copyright: Option<String>,
    /// `metadata.keywords`.
    pub keywords: Option<Vec<Keyword>>,
    /// `metadata.sidecar_naming`.
    pub sidecar_naming: Option<SidecarNaming>,
}

impl MetadataKeys {
    /// Reads the mapping `metadata` of `config`, which passed the strict reading.
    ///
    /// # Errors
    ///
    /// The first invalid keyword, at its value.
    pub(crate) fn from_config(config: &ConfigFile) -> Result<Self, ConfigError> {
        let Some(metadata) = config.values().mapping(METADATA_KEY.name()) else {
            return Ok(MetadataKeys::default());
        };
        Ok(MetadataKeys {
            creator: text_list(metadata, "creator"),
            copyright: metadata.text("copyright").map(|text| text.value.clone()),
            keywords: metadata
                .text_list("keywords")
                .map(|entries| keywords(config, entries))
                .transpose()?,
            sidecar_naming: metadata
                .text("sidecar_naming")
                .map(|naming| sidecar_naming(&naming.value)),
        })
    }

    /// These keys over `base`: each key set here replaces the one of `base`
    /// (IMPORT-019, "Values").
    #[must_use]
    pub fn over(&self, base: &MetadataKeys) -> MetadataKeys {
        MetadataKeys {
            creator: self.creator.clone().or_else(|| base.creator.clone()),
            copyright: self.copyright.clone().or_else(|| base.copyright.clone()),
            keywords: self.keywords.clone().or_else(|| base.keywords.clone()),
            sidecar_naming: self.sidecar_naming.or(base.sidecar_naming),
        }
    }

    /// The values with every default applied.
    #[must_use]
    pub fn values(&self) -> ImportMetadata {
        ImportMetadata {
            creator: self.creator.clone().unwrap_or_default(),
            copyright: self.copyright.clone(),
            keywords: self.keywords.clone().unwrap_or_default(),
            sidecar_naming: self.sidecar_naming.unwrap_or_default(),
        }
    }
}

/// The metadata an import writes to each photo's sidecar besides its job code
/// (IMPORT-019, ADR 0040 §2).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ImportMetadata {
    /// The creators, in order (`dc:creator`).
    pub creator: Vec<String>,
    /// The copyright notice, `{year}` not yet replaced (`dc:rights`).
    pub copyright: Option<String>,
    /// The keywords (`dc:subject`, `lr:hierarchicalSubject`, `mwg-kw:Keywords`).
    pub keywords: Vec<Keyword>,
    /// How the sidecars are named.
    pub sidecar_naming: SidecarNaming,
}

/// Metadata without any value, for imports that write no sidecars.
static NO_METADATA: ImportMetadata = ImportMetadata {
    creator: Vec::new(),
    copyright: None,
    keywords: Vec::new(),
    sidecar_naming: SidecarNaming::OriginalWithExtension,
};

impl ImportMetadata {
    /// Metadata without any value.
    #[must_use]
    pub fn none() -> &'static ImportMetadata {
        &NO_METADATA
    }

    /// Whether any value is set; the sidecar naming is not a value.
    #[must_use]
    pub fn has_values(&self) -> bool {
        !self.creator.is_empty() || self.copyright.is_some() || !self.keywords.is_empty()
    }

    /// The copyright notice with `{year}` replaced by `year`.
    #[must_use]
    pub fn copyright_of(&self, year: i32) -> Option<String> {
        self.copyright
            .as_ref()
            .map(|copyright| copyright.replace(YEAR_TOKEN, &year.to_string()))
    }
}

fn text_list(values: &Values, key: &str) -> Option<Vec<String>> {
    values
        .text_list(key)
        .map(|entries| entries.iter().map(|entry| entry.value.clone()).collect())
}

fn keywords(config: &ConfigFile, entries: &[Located<String>]) -> Result<Vec<Keyword>, ConfigError> {
    entries
        .iter()
        .map(|entry| {
            Keyword::parse(&entry.value)
                .map_err(|error| config.error_at(entry.position, error.to_string()))
        })
        .collect()
}

/// The naming of a choice the strict reading accepted.
fn sidecar_naming(text: &str) -> SidecarNaming {
    match text {
        ORIGINAL_WITHOUT_EXTENSION => SidecarNaming::OriginalWithoutExtension,
        _ => SidecarNaming::OriginalWithExtension,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keyword(text: &str) -> Keyword {
        Keyword::parse(text).unwrap_or_else(|error| panic!("{text}: {error}"))
    }

    /// spec: IMPORT-019.5
    #[test]
    fn rejects_empty_levels_and_pipes() {
        assert_eq!(Keyword::parse(""), Err(KeywordError::Empty));
        for text in ["/Holiday", "Holiday/", "Holiday//Beach", "/"] {
            assert_eq!(
                Keyword::parse(text),
                Err(KeywordError::EmptyLevel),
                "{text}"
            );
        }
        assert_eq!(Keyword::parse("Holiday|Beach"), Err(KeywordError::Pipe));
        assert_eq!(keyword("Holiday/Beach").levels(), ["Holiday", "Beach"]);
    }

    /// spec: IMPORT-019.3
    #[test]
    fn the_profile_replaces_each_key_it_sets() {
        let config = MetadataKeys {
            creator: Some(vec!["Andy Pollari".to_owned()]),
            keywords: Some(vec![keyword("Family")]),
            ..MetadataKeys::default()
        };
        let profile = MetadataKeys {
            keywords: Some(vec![keyword("Holiday")]),
            ..MetadataKeys::default()
        };
        let values = profile.over(&config).values();
        assert_eq!(values.creator, ["Andy Pollari"]);
        assert_eq!(values.keywords, [keyword("Holiday")]);
        assert_eq!(values.sidecar_naming, SidecarNaming::OriginalWithExtension);
    }

    #[test]
    fn replaces_every_year_token() {
        let metadata = ImportMetadata {
            copyright: Some("© {year} Andy Pollari, {year}".to_owned()),
            ..ImportMetadata::default()
        };
        assert_eq!(
            metadata.copyright_of(2026).as_deref(),
            Some("© 2026 Andy Pollari, 2026")
        );
        assert!(metadata.has_values());
        assert!(!ImportMetadata::none().has_values());
    }
}
