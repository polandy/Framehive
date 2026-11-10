// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

/// The role of one file of an asset (glossary: Asset file).
///
/// An asset is an image or video together with all its associated files; each of
/// those files has exactly one role.
///
/// ```
/// use domain::AssetFileRole;
///
/// let role = AssetFileRole::Sidecar;
/// assert_ne!(role, AssetFileRole::Raw);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AssetFileRole {
    /// The camera's RAW file.
    Raw,
    /// An image the camera rendered, such as a JPEG or HEIF file.
    Rendered,
    /// A video file.
    Video,
    /// An XMP file next to the original, carrier of the metadata; also a camera's
    /// video thumbnail file (`.THM`).
    Sidecar,
}

/// The extensions of RAW files, in upper case (requirements 2.7).
pub const RAW_EXTENSIONS: [&str; 22] = [
    "3FR", "ARW", "CR2", "CR3", "CRW", "DCR", "DNG", "FFF", "IIQ", "MEF", "MOS", "MRW", "NEF",
    "NRW", "ORF", "PEF", "RAF", "RAW", "RW2", "SR2", "SRW", "X3F",
];

/// The extensions of video files, in upper case (requirements 2.7).
pub const VIDEO_EXTENSIONS: [&str; 12] = [
    "3GP", "AVI", "M2T", "M2TS", "MOV", "MP4", "MPEG", "MPG", "MOD", "TOD", "MTS", "MKV",
];
