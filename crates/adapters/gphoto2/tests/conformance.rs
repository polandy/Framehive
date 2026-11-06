// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Port conformance test: [`Gphoto2Cameras`] passes the conformance suite of the
//! `files::Cameras` port, and with it the `ReadOnlyFileSystem` suite, through
//! libgphoto2's `directory` driver, which serves a temporary folder as a camera on the
//! port `disk:<folder>` (CLAUDE.md, "Adding things"; ADR 0034 §3).

mod support;

use adapter_gphoto2::Gphoto2Cameras;
use files::testing::{CamerasUnderTest, check_cameras};

/// A USB port at which no camera is connected: bus and device numbers are at most 127.
const MISSING_PORT: &str = "usb:999,999";

#[test]
fn conforms_to_the_cameras_port() {
    let camera = support::FolderCamera::new();
    let failures = check_cameras(&CamerasUnderTest {
        cameras: &Gphoto2Cameras::new(),
        port: &camera.port(),
        missing_port: MISSING_PORT,
        add_folder: &|path| camera.add_folder(path),
        add_file: &|path, content, modified| camera.add_file(path, content, modified),
    });
    assert!(failures.is_empty(), "{failures:#?}");
}
