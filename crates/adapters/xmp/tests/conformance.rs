// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Port conformance test: [`QuickXml`] passes the conformance suite of the
//! `metadata::xmp::XmlSyntax` port (CLAUDE.md, "Adding things").

use adapter_xmp::QuickXml;
use metadata::testing::check_xml_syntax;

/// spec: CORE-009.11
/// spec: CORE-009.12
#[test]
fn conforms_to_the_xml_syntax_port() {
    let failures = check_xml_syntax(&QuickXml::new());
    assert!(failures.is_empty(), "{failures:#?}");
}
