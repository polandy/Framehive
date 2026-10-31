// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Rendering a name template for one asset file (IMPORT-001).

use domain::LocalDateTime;
use thiserror::Error;

use crate::NameTemplate;
use crate::template::Part;
use crate::token::Token;

/// Characters that exFAT, SMB or macOS do not accept in a name.
const INCOMPATIBLE_CHARACTERS: [char; 9] = ['<', '>', ':', '"', '\\', '|', '?', '*', '\u{7f}'];

/// The values every file of one asset is rendered from (IMPORT-001, "Assets").
///
/// Pure data: the caller resolves the capture time, e.g. with
/// [`asset_date`](crate::asset_date), and fixes the import time when the job is queued.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetValues {
    /// The base name the files of the asset share, e.g. `DSC_0042` (grouping,
    /// LIBRARY-001). It gives `{name}` and the image number of `{number}`.
    pub base_name: String,
    /// The capture time of the asset, after camera time correction, or the file
    /// modification time when no file has one. It gives `{date:…}`.
    pub capture_time: LocalDateTime,
    /// The time the import was queued, in the machine's local time. It gives
    /// `{imported:…}`.
    pub import_time: LocalDateTime,
}

/// Whether rendered names are kept valid on exFAT, SMB and macOS.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum CompatibilityMode {
    /// Replace `< > : " \ | ? *` and control characters with `_`, and remove trailing
    /// dots and spaces, in every segment.
    #[default]
    On,
    /// Use the values unchanged.
    Off,
}

/// Why one file cannot be named; the other files of an import are not affected.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum RenderError {
    /// `{number}` is used, but the base name does not end in a digit.
    #[error("{token} needs an image number, but {base_name:?} does not end in a digit")]
    NoImageNumber {
        /// The image number token as written in the template.
        token: String,
        /// The base name without an image number.
        base_name: String,
    },
    /// The file name is not the asset's base name, a dot and an extension.
    #[error("{file_name:?} is not a file of the asset {base_name:?}")]
    FileNotInAsset {
        /// The rejected file name.
        file_name: String,
        /// The base name of the asset.
        base_name: String,
    },
    /// A rendered segment is empty, `.` or `..`, so it would not stay inside the
    /// destination.
    #[error("the rendered segment {segment:?} is not a valid folder or file name")]
    InvalidSegment {
        /// The segment as rendered.
        segment: String,
    },
    /// The template has a sequence token; sequence numbers are specified by IMPORT-002
    /// and not available yet.
    #[error("{token} cannot be rendered yet; sequence numbers are not available")]
    SequenceUnavailable {
        /// The sequence token as written in the template.
        token: String,
    },
}

impl NameTemplate {
    /// Renders the path of one file of an asset, relative to the import destination,
    /// with `/` between the segments.
    ///
    /// The extension is everything after the asset's base name and a dot, so a sidecar
    /// keeps its suffix and follows its file: `DSC_0042.RAF.xmp` has the extension
    /// `RAF.xmp`.
    ///
    /// ```
    /// use domain::LocalDateTime;
    /// use naming::{AssetValues, CompatibilityMode, NameTemplate};
    ///
    /// let template = NameTemplate::parse("{date:%Y-%m-%d}/{name}.{ext}")?;
    /// let values = AssetValues {
    ///     base_name: "DSC_0042".to_owned(),
    ///     capture_time: LocalDateTime::new(2026, 10, 17, 14, 5, 9)?,
    ///     import_time: LocalDateTime::new(2026, 10, 19, 9, 0, 0)?,
    /// };
    /// let path = template.render(&values, "DSC_0042.RAF.xmp", CompatibilityMode::On)?;
    /// assert_eq!(path, "2026-10-17/DSC_0042.RAF.xmp");
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    ///
    /// Returns a [`RenderError`] when this file cannot be named.
    pub fn render(
        &self,
        values: &AssetValues,
        file_name: &str,
        mode: CompatibilityMode,
    ) -> Result<String, RenderError> {
        let extension = extension_in_asset(&values.base_name, file_name)?;
        let mut segments = Vec::with_capacity(self.segments().len());
        for parts in self.segments() {
            let mut segment = String::new();
            for part in parts {
                render_part(part, values, extension, &mut segment)?;
            }
            segments.push(finish_segment(segment, mode)?);
        }
        Ok(segments.join("/"))
    }
}

fn extension_in_asset<'a>(base_name: &str, file_name: &'a str) -> Result<&'a str, RenderError> {
    file_name
        .strip_prefix(base_name)
        .and_then(|rest| rest.strip_prefix('.'))
        .filter(|extension| !extension.is_empty())
        .ok_or_else(|| RenderError::FileNotInAsset {
            file_name: file_name.to_owned(),
            base_name: base_name.to_owned(),
        })
}

fn render_part(
    part: &Part,
    values: &AssetValues,
    extension: &str,
    output: &mut String,
) -> Result<(), RenderError> {
    match part {
        Part::Literal(text) => output.push_str(text),
        Part::Token(Token::CaptureTime(format)) => format.render(values.capture_time, output),
        Part::Token(Token::ImportTime(format)) => format.render(values.import_time, output),
        Part::Token(Token::BaseName(case)) => output.push_str(&case.apply(&values.base_name)),
        Part::Token(Token::Extension(case)) => output.push_str(&case.apply(extension)),
        Part::Token(Token::ImageNumber { digits, text }) => {
            output.push_str(&image_number(&values.base_name, *digits, text)?);
        }
        Part::Token(Token::Sequence { text }) => {
            return Err(RenderError::SequenceUnavailable {
                token: text.clone(),
            });
        }
    }
    Ok(())
}

/// The trailing digits of `base_name`, or their last `digits`, padded with zeros.
fn image_number(
    base_name: &str,
    digits: Option<usize>,
    token: &str,
) -> Result<String, RenderError> {
    let number_start = base_name
        .trim_end_matches(|c: char| c.is_ascii_digit())
        .len();
    let number = &base_name[number_start..];
    if number.is_empty() {
        return Err(RenderError::NoImageNumber {
            token: token.to_owned(),
            base_name: base_name.to_owned(),
        });
    }
    Ok(match digits {
        None => number.to_owned(),
        Some(width) => {
            // The number is ASCII digits, so byte and character positions agree.
            let last = &number[number.len().saturating_sub(width)..];
            format!("{last:0>width$}")
        }
    })
}

fn finish_segment(segment: String, mode: CompatibilityMode) -> Result<String, RenderError> {
    let segment = match mode {
        CompatibilityMode::On => make_compatible(&segment),
        CompatibilityMode::Off => segment,
    };
    if segment.is_empty() || segment == "." || segment == ".." {
        return Err(RenderError::InvalidSegment { segment });
    }
    Ok(segment)
}

fn make_compatible(segment: &str) -> String {
    let replaced: String = segment
        .chars()
        .map(|c| {
            if c < ' ' || INCOMPATIBLE_CHARACTERS.contains(&c) {
                '_'
            } else {
                c
            }
        })
        .collect();
    replaced.trim_end_matches(['.', ' ']).to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn values() -> AssetValues {
        let time = LocalDateTime::new(2026, 10, 17, 14, 5, 9).unwrap();
        AssetValues {
            base_name: "DSC_0042".to_owned(),
            capture_time: time,
            import_time: time,
        }
    }

    #[test]
    fn sequence_tokens_are_not_rendered_yet() {
        let template = NameTemplate::parse("{seq_today:3}.{ext}").unwrap();

        let error = template.render(&values(), "DSC_0042.RAF", CompatibilityMode::On);

        assert_eq!(
            error,
            Err(RenderError::SequenceUnavailable {
                token: "{seq_today:3}".to_owned()
            })
        );
    }

    #[test]
    fn literal_dot_segment_from_values_is_refused_without_compatibility_mode() {
        let template = NameTemplate::parse("{name}/x.{ext}").unwrap();
        let values = AssetValues {
            base_name: "..".to_owned(),
            ..values()
        };

        let error = template.render(&values, "...jpg", CompatibilityMode::Off);

        assert_eq!(
            error,
            Err(RenderError::InvalidSegment {
                segment: "..".to_owned()
            })
        );
    }
}
