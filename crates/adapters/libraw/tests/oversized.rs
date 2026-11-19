// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! A file above the size limit never reaches LibRaw (PREVIEWS-001.9).

#![allow(clippy::unwrap_used, reason = "test setup may panic")]

use std::fs::File;
use std::path::Path;

use adapter_libraw::{LibRawDecoder, MAX_FILE_SIZE};
use previews::{DecodeError, DecodeMode, PreviewDecoder};

/// spec: PREVIEWS-001.9
#[test]
fn a_file_above_the_size_limit_fails_without_being_read() {
    let path = Path::new(env!("CARGO_TARGET_TMPDIR")).join("oversized.RAF");
    // A sparse file: it takes no disk space, and a read would take seconds.
    File::create(&path)
        .unwrap()
        .set_len(MAX_FILE_SIZE + 1)
        .unwrap();

    let error = LibRawDecoder::new()
        .decode_full(&path, DecodeMode::Interactive)
        .unwrap_err();

    match error {
        DecodeError::Failed {
            path: failed,
            message,
        } => {
            assert_eq!(failed, path);
            assert_eq!(message, "the file is larger than 512 MiB");
        }
        other => panic!("expected Failed, got {other:?}"),
    }
}
