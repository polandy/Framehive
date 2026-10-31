// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The name template language of Framehive (IMPORT-001).
//!
//! A [`NameTemplate`] derives an imported file's folders and name. This service
//! validates templates and renders them as a pure function of the values it is given:
//! no file system, no wall clock (architecture §3).

#![forbid(unsafe_code)]

mod date_format;
mod dating;
mod error;
mod render;
mod template;
mod token;

pub use dating::{AssetDate, DateSource, FileTimes, asset_date};
pub use error::{TemplateError, TemplateErrorKind};
pub use render::{AssetValues, CompatibilityMode, RenderError};
pub use template::NameTemplate;
