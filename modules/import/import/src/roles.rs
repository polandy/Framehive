// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The role of a file from its extension: the formats of requirements section 2.7
//! (IMPORT-005).

use domain::{AssetFileRole, RAW_EXTENSIONS, VIDEO_EXTENSIONS};

const RENDERED: [&str; 11] = [
    "JPG", "JPEG", "HEIF", "HEIC", "HIF", "TIF", "TIFF", "MPO", "AVIF", "JXL", "PNG",
];

const SIDECAR: [&str; 2] = ["XMP", "THM"];

/// The role of a file with `extension` (without the `.`), ignoring case, or `None`
/// for a format the import does not support.
pub(crate) fn role_for_extension(extension: &str) -> Option<AssetFileRole> {
    let lists: [(&[&str], AssetFileRole); 4] = [
        (&RAW_EXTENSIONS, AssetFileRole::Raw),
        (&RENDERED, AssetFileRole::Rendered),
        (&VIDEO_EXTENSIONS, AssetFileRole::Video),
        (&SIDECAR, AssetFileRole::Sidecar),
    ];
    lists
        .into_iter()
        .find(|(extensions, _)| {
            extensions
                .iter()
                .any(|known| known.eq_ignore_ascii_case(extension))
        })
        .map(|(_, role)| role)
}
