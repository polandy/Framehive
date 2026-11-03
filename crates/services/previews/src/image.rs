// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The [`DecodedImage`] of a full decode.

use crate::ImageLayoutError;

/// Samples per pixel: red, green and blue.
const CHANNELS: usize = 3;

/// The result of a full decode (PREVIEWS-001): three 16-bit sRGB samples per pixel,
/// red, green and blue, row by row from the top left, not rotated by the file's
/// `Orientation` tag.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedImage {
    width: u32,
    height: u32,
    samples: Vec<u16>,
}

impl DecodedImage {
    /// An image of `width` × `height` pixels with `samples` in the order above.
    ///
    /// # Errors
    ///
    /// Returns an [`ImageLayoutError`] if a side is zero or `samples` does not hold
    /// exactly width × height × 3 samples.
    pub fn new(width: u32, height: u32, samples: Vec<u16>) -> Result<Self, ImageLayoutError> {
        let expected = sample_count(width, height);
        if width == 0 || height == 0 || expected != Some(samples.len()) {
            return Err(ImageLayoutError {
                width,
                height,
                samples: samples.len(),
            });
        }
        Ok(DecodedImage {
            width,
            height,
            samples,
        })
    }

    /// The width in pixels.
    #[must_use]
    pub fn width(&self) -> u32 {
        self.width
    }

    /// The height in pixels.
    #[must_use]
    pub fn height(&self) -> u32 {
        self.height
    }

    /// The samples, three per pixel, row by row.
    #[must_use]
    pub fn samples(&self) -> &[u16] {
        &self.samples
    }

    /// The samples as two bytes each in little-endian order: the input of the pixel
    /// hash (PREVIEWS-001).
    #[must_use]
    pub fn sample_bytes(&self) -> Vec<u8> {
        self.samples
            .iter()
            .flat_map(|sample| sample.to_le_bytes())
            .collect()
    }
}

fn sample_count(width: u32, height: u32) -> Option<usize> {
    let width = usize::try_from(width).ok()?;
    let height = usize::try_from(height).ok()?;
    width.checked_mul(height)?.checked_mul(CHANNELS)
}
