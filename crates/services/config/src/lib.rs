// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Strict reading of Framehive's YAML configuration files (CORE-006, ADR 0011,
//! ADR 0032).
//!
//! A file type describes its keys with [`FileType`] and [`Key`]. [`read`] turns the
//! text of one file into [`ConfigFile`], whose [`Values`] give each value with its
//! [`Position`], so the owner of the file type can check values further and report an
//! error at the value with [`ConfigFile::error_at`]. [`json_schema`] generates the
//! JSON Schema of a file type from the same description.
//!
//! The YAML text itself is parsed behind the [`YamlParser`] port into a [`YamlNode`]
//! tree; `adapter-yaml` implements it. This crate does no I/O (architecture §3).

#![forbid(unsafe_code)]

mod core_schema;
mod description;
mod error;
mod parser;
mod read;
mod schema;
mod suggest;
pub mod testing;
mod tree;
mod values;

pub use description::{DefaultValue, FileType, Key, KeyKind};
pub use error::{ConfigError, Kind, Problem};
pub use parser::{YamlParseError, YamlParseErrorKind, YamlParser};
pub use read::read;
pub use schema::json_schema;
pub use tree::{Position, ScalarStyle, YamlEntry, YamlNode, YamlValue};
pub use values::{ConfigFile, Located, Values};
