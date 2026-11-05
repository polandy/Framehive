// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Port conformance test: [`SaphyrParser`] passes the conformance suite of the
//! `config::YamlParser` port (CLAUDE.md, "Adding things").

use adapter_yaml::SaphyrParser;
use config::testing::check_yaml_parser;

/// spec: CORE-006.5
/// spec: CORE-006.7
/// spec: CORE-006.8
#[test]
fn conforms_to_the_yaml_parser_port() {
    let failures = check_yaml_parser(&SaphyrParser::new());
    assert!(failures.is_empty(), "{failures:#?}");
}
