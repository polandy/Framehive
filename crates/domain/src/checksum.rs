// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

use std::fmt::{self, Write as _};
use std::str::FromStr;

use thiserror::Error;

/// Hash of a file's content together with its algorithm (glossary: Checksum).
///
/// It is written `<algorithm>:<hex>`, for example `blake3:…` (architecture §8). Two
/// checksums are equal only if both the algorithm and the digest are equal, so a
/// checksum of one algorithm never matches one of another.
///
/// ```
/// use domain::Checksum;
///
/// let checksum: Checksum = "blake3:00ff10".parse()?;
/// assert_eq!(checksum.algorithm(), "blake3");
/// assert_eq!(checksum.digest(), &[0x00, 0xff, 0x10]);
/// assert_eq!(checksum.to_string(), "blake3:00ff10");
/// # Ok::<(), domain::ChecksumError>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Checksum {
    algorithm: String,
    digest: Vec<u8>,
}

/// Error returned when values do not form a valid [`Checksum`].
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ChecksumError {
    /// The algorithm id is empty, or not lowercase ASCII letters and digits starting
    /// with a letter.
    #[error(
        "a checksum algorithm is lowercase letters and digits, starting with a letter, got {algorithm:?}"
    )]
    InvalidAlgorithm {
        /// The rejected algorithm id.
        algorithm: String,
    },
    /// The digest has no bytes.
    #[error("a checksum digest must not be empty")]
    EmptyDigest,
    /// The written form has no `:` between algorithm and digest.
    #[error("a checksum is written <algorithm>:<hex>, got {text:?}")]
    MissingSeparator {
        /// The rejected text.
        text: String,
    },
    /// The digest is not an even number of hexadecimal digits.
    #[error("a checksum digest is an even number of hexadecimal digits, got {digest:?}")]
    InvalidHex {
        /// The rejected digest text.
        digest: String,
    },
}

impl Checksum {
    /// The algorithm id of BLAKE3, the checksum of Framehive (architecture §8).
    pub const BLAKE3: &'static str = "blake3";

    /// Creates a checksum from an algorithm id and the digest bytes.
    ///
    /// # Errors
    ///
    /// Returns [`ChecksumError::InvalidAlgorithm`] for an algorithm id that is not
    /// lowercase ASCII letters and digits starting with a letter, and
    /// [`ChecksumError::EmptyDigest`] for an empty digest.
    pub fn new(algorithm: &str, digest: Vec<u8>) -> Result<Self, ChecksumError> {
        if !is_valid_algorithm(algorithm) {
            return Err(ChecksumError::InvalidAlgorithm {
                algorithm: algorithm.to_owned(),
            });
        }
        if digest.is_empty() {
            return Err(ChecksumError::EmptyDigest);
        }
        Ok(Checksum {
            algorithm: algorithm.to_owned(),
            digest,
        })
    }

    /// Returns the algorithm id, such as `blake3`.
    #[must_use]
    pub fn algorithm(&self) -> &str {
        &self.algorithm
    }

    /// Returns the digest bytes.
    #[must_use]
    pub fn digest(&self) -> &[u8] {
        &self.digest
    }
}

fn is_valid_algorithm(algorithm: &str) -> bool {
    let mut characters = algorithm.chars();
    characters
        .next()
        .is_some_and(|first| first.is_ascii_lowercase())
        && characters.all(|character| character.is_ascii_lowercase() || character.is_ascii_digit())
}

impl fmt::Display for Checksum {
    /// Writes `<algorithm>:<hex>` with lowercase hexadecimal digits.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.algorithm)?;
        formatter.write_char(':')?;
        for byte in &self.digest {
            write!(formatter, "{byte:02x}")?;
        }
        Ok(())
    }
}

impl FromStr for Checksum {
    type Err = ChecksumError;

    /// Reads `<algorithm>:<hex>`. Uppercase hexadecimal digits are accepted when
    /// reading; writing always uses lowercase.
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let (algorithm, hex) =
            text.split_once(':')
                .ok_or_else(|| ChecksumError::MissingSeparator {
                    text: text.to_owned(),
                })?;
        let digest = decode_hex(hex).ok_or_else(|| ChecksumError::InvalidHex {
            digest: hex.to_owned(),
        })?;
        Checksum::new(algorithm, digest)
    }
}

fn decode_hex(hex: &str) -> Option<Vec<u8>> {
    if !hex.len().is_multiple_of(2) {
        return None;
    }
    hex.as_bytes()
        .chunks(2)
        .map(|pair| {
            let high = char::from(pair[0]).to_digit(16)?;
            let low = char::from(pair[1]).to_digit(16)?;
            u8::try_from(high * 16 + low).ok()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_written_as_algorithm_and_lowercase_hex() {
        let checksum = Checksum::new("blake3", vec![0xab, 0x01, 0xcd]).unwrap();
        assert_eq!(checksum.to_string(), "blake3:ab01cd");
    }

    #[test]
    fn written_form_reads_back_unchanged() {
        let checksum = Checksum::new("fnv", vec![0, 1, 2, 254, 255]).unwrap();
        assert_eq!(checksum.to_string().parse::<Checksum>(), Ok(checksum));
    }

    #[test]
    fn reading_accepts_uppercase_hex() {
        let checksum: Checksum = "blake3:ABcd".parse().unwrap();
        assert_eq!(checksum.digest(), &[0xab, 0xcd]);
        assert_eq!(checksum.to_string(), "blake3:abcd");
    }

    #[test]
    fn algorithm_is_part_of_equality() {
        let blake3 = Checksum::new("blake3", vec![7]).unwrap();
        let fnv = Checksum::new("fnv", vec![7]).unwrap();
        assert_ne!(blake3, fnv);
    }

    #[test]
    fn rejects_invalid_algorithm_ids() {
        for algorithm in ["", "BLAKE3", "3blake", "bla-ke", "fnv:"] {
            assert_eq!(
                Checksum::new(algorithm, vec![1]),
                Err(ChecksumError::InvalidAlgorithm {
                    algorithm: algorithm.to_owned()
                }),
                "{algorithm:?}"
            );
        }
    }

    #[test]
    fn rejects_an_empty_digest() {
        assert_eq!(
            Checksum::new("blake3", Vec::new()),
            Err(ChecksumError::EmptyDigest)
        );
        assert_eq!(
            "blake3:".parse::<Checksum>(),
            Err(ChecksumError::EmptyDigest)
        );
    }

    #[test]
    fn rejects_text_without_separator() {
        assert_eq!(
            "blake3abcd".parse::<Checksum>(),
            Err(ChecksumError::MissingSeparator {
                text: "blake3abcd".to_owned()
            })
        );
    }

    #[test]
    fn rejects_odd_or_non_hex_digests() {
        for digest in ["abc", "zz", "a:", "é0"] {
            assert_eq!(
                format!("blake3:{digest}").parse::<Checksum>(),
                Err(ChecksumError::InvalidHex {
                    digest: digest.to_owned()
                }),
                "{digest:?}"
            );
        }
    }
}
