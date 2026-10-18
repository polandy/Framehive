// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

use thiserror::Error;

/// Star rating of an asset, 0 to 5 stars (glossary: Rating).
///
/// Zero stars means "not rated". Rejecting an asset is a pick state, not a rating.
///
/// ```
/// use domain::Rating;
///
/// let rating = Rating::new(4)?;
/// assert_eq!(rating.stars(), 4);
/// # Ok::<(), domain::RatingError>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Rating(u8);

/// Error returned when a value is not a valid [`Rating`].
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum RatingError {
    /// The number of stars is above the maximum of five.
    #[error("a rating has at most {max} stars, got {stars}", max = Rating::MAX_STARS)]
    TooManyStars {
        /// The rejected number of stars.
        stars: u8,
    },
}

impl Rating {
    /// The highest number of stars.
    pub const MAX_STARS: u8 = 5;

    /// An asset without a rating.
    pub const UNRATED: Rating = Rating(0);

    /// Creates a rating with the given number of stars.
    ///
    /// # Errors
    ///
    /// Returns [`RatingError::TooManyStars`] if `stars` is greater than five.
    pub fn new(stars: u8) -> Result<Self, RatingError> {
        if stars > Self::MAX_STARS {
            return Err(RatingError::TooManyStars { stars });
        }
        Ok(Rating(stars))
    }

    /// Returns the number of stars, 0 to 5.
    #[must_use]
    pub fn stars(self) -> u8 {
        self.0
    }

    /// Returns `true` if the asset has no rating.
    #[must_use]
    pub fn is_unrated(self) -> bool {
        self == Self::UNRATED
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_zero_to_five_stars() {
        for stars in 0..=5 {
            assert_eq!(Rating::new(stars).unwrap().stars(), stars);
        }
    }

    #[test]
    fn rejects_more_than_five_stars() {
        assert_eq!(Rating::new(6), Err(RatingError::TooManyStars { stars: 6 }));
        assert_eq!(
            Rating::new(u8::MAX),
            Err(RatingError::TooManyStars { stars: u8::MAX })
        );
    }

    #[test]
    fn default_is_unrated() {
        assert!(Rating::default().is_unrated());
        assert!(!Rating::new(1).unwrap().is_unrated());
    }

    #[test]
    fn error_message_names_the_limit() {
        let message = Rating::new(7).unwrap_err().to_string();
        assert_eq!(message, "a rating has at most 5 stars, got 7");
    }
}
