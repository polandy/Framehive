// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The conformance suite of the [`XmlSyntax`] port (CLAUDE.md, "Adding things").

use crate::xmp::{
    XmlAttribute, XmlEvent, XmlName, XmlStart, XmlSyntax, XmlSyntaxError, XmlSyntaxErrorKind,
};

const NAMESPACE: &str = "urn:example:a";

/// Runs every check against `xml` and returns one message per failed check; an
/// empty list means the tokenizer conforms.
#[must_use]
pub fn check_xml_syntax(xml: &dyn XmlSyntax) -> Vec<String> {
    CHECKS
        .iter()
        .filter_map(|(name, check)| check(xml).err().map(|failure| format!("{name}: {failure}")))
        .collect()
}

type Check = fn(&dyn XmlSyntax) -> Result<(), String>;

const CHECKS: [(&str, Check); 10] = [
    ("names are resolved to their namespace", names),
    ("namespace names are unescaped", escaped_namespace),
    ("an empty element is a start and an end", empty_element),
    (
        "declarations, comments and instructions are skipped",
        skipped,
    ),
    ("references, CDATA and line ends become text", text_forms),
    ("attribute values are normalized", attribute_values),
    ("a document type declaration is refused", document_type),
    ("malformed text is refused within the text", malformed),
    ("events are written as given", written),
    ("written text and attributes read back", round_trip),
];

fn names(xml: &dyn XmlSyntax) -> Result<(), String> {
    let text = r#"<p:a xmlns:p="urn:example:a" xmlns="urn:example:b" p:x="1" y="2"><b/></p:a>"#;
    let expected = vec![
        XmlEvent::Start(XmlStart {
            name: XmlName::new(NAMESPACE, "p", "a"),
            attributes: vec![
                attribute(XmlName::new(NAMESPACE, "p", "x"), "1"),
                attribute(XmlName::new("", "", "y"), "2"),
            ],
            namespaces: vec![
                ("p".to_owned(), NAMESPACE.to_owned()),
                (String::new(), "urn:example:b".to_owned()),
            ],
        }),
        start(XmlName::new("urn:example:b", "", "b")),
        XmlEvent::End,
        XmlEvent::End,
    ];
    expect_events(xml, text, &expected)
}

fn escaped_namespace(xml: &dyn XmlSyntax) -> Result<(), String> {
    let text = r#"<p:a xmlns:p="urn:a&amp;b"/>"#;
    let expected = [
        XmlEvent::Start(XmlStart {
            name: XmlName::new("urn:a&b", "p", "a"),
            attributes: Vec::new(),
            namespaces: vec![("p".to_owned(), "urn:a&b".to_owned())],
        }),
        XmlEvent::End,
    ];
    expect_events(xml, text, &expected)
}

fn empty_element(xml: &dyn XmlSyntax) -> Result<(), String> {
    expect_events(xml, "<a/>", &[start(plain("a")), XmlEvent::End])
}

fn skipped(xml: &dyn XmlSyntax) -> Result<(), String> {
    let text = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<?xpacket begin=\"\"?><!-- c --><a/><?xpacket end=\"w\"?>";
    let events = read(xml, text)?;
    let elements: Vec<_> = events
        .into_iter()
        .filter(|event| !matches!(event, XmlEvent::Text(text) if text.trim().is_empty()))
        .collect();
    expect_equal(
        text,
        elements.as_slice(),
        &[start(plain("a")), XmlEvent::End][..],
    )
}

fn text_forms(xml: &dyn XmlSyntax) -> Result<(), String> {
    let text = "<a>1 &lt;&amp;&gt;&quot;&apos; &#65;&#x42;<![CDATA[<c>]]>\r\nd</a>";
    let events = read(xml, text)?;
    let joined: String = events
        .iter()
        .filter_map(|event| match event {
            XmlEvent::Text(text) => Some(text.as_str()),
            _ => None,
        })
        .collect();
    expect_equal(text, &joined.as_str(), &"1 <&>\"' AB<c>\nd")
}

fn attribute_values(xml: &dyn XmlSyntax) -> Result<(), String> {
    let text = "<a b=\"x\ny\tz&#10;&amp;\"/>";
    let expected = [
        XmlEvent::Start(XmlStart {
            name: plain("a"),
            attributes: vec![attribute(plain("b"), "x y z\n&")],
            namespaces: Vec::new(),
        }),
        XmlEvent::End,
    ];
    expect_events(xml, text, &expected)
}

fn document_type(xml: &dyn XmlSyntax) -> Result<(), String> {
    let text = "<!DOCTYPE a [<!ENTITY e \"boom\">]><a>&e;</a>";
    match xml.read(text) {
        Err(XmlSyntaxError {
            kind: XmlSyntaxErrorKind::DocumentType,
            ..
        }) => Ok(()),
        other => Err(format!("{text:?} gave {other:?}")),
    }
}

fn malformed(xml: &dyn XmlSyntax) -> Result<(), String> {
    for text in [
        "<a><b></a>",
        "<a>",
        "<a>&unknown;</a>",
        "<p:a/>",
        "<a x=\"1\" x=\"2\"/>",
        "<a",
        "<a b=\"<\"/>",
        "<a b/=\"1\"/>",
        "<a:b:c xmlns:a=\"urn:a\"/>",
        "<1a/>",
        "<a xmlns:1=\"urn:a\"/>",
    ] {
        match xml.read(text) {
            Err(XmlSyntaxError {
                offset,
                kind: XmlSyntaxErrorKind::Malformed(_),
            }) if offset <= text.len() => {}
            other => return Err(format!("{text:?} gave {other:?}")),
        }
    }
    Ok(())
}

fn written(xml: &dyn XmlSyntax) -> Result<(), String> {
    let events = [
        XmlEvent::Start(XmlStart {
            name: XmlName::new(NAMESPACE, "p", "a"),
            attributes: vec![attribute(plain("x"), "1")],
            namespaces: vec![("p".to_owned(), NAMESPACE.to_owned())],
        }),
        XmlEvent::Text("\n  ".to_owned()),
        start(XmlName::new(NAMESPACE, "p", "b")),
        XmlEvent::End,
        start(plain("c")),
        XmlEvent::Text(String::new()),
        XmlEvent::End,
        XmlEvent::End,
    ];
    let expected = "<p:a x=\"1\" xmlns:p=\"urn:example:a\">\n  <p:b/><c></c></p:a>";
    expect_equal("the events", &xml.write(&events).as_str(), &expected)
}

fn round_trip(xml: &dyn XmlSyntax) -> Result<(), String> {
    let awkward = "<&>\"' ]]> \r\n\t end";
    let events = vec![
        XmlEvent::Start(XmlStart {
            name: plain("a"),
            attributes: vec![attribute(plain("b"), awkward)],
            namespaces: Vec::new(),
        }),
        XmlEvent::Text(awkward.to_owned()),
        XmlEvent::End,
    ];
    let text = xml.write(&events);
    let read_back = read(xml, &text)?;
    let joined = join_texts(read_back);
    expect_equal(&text, &joined, &events)
}

/// Joins adjacent text events, which a tokenizer may split.
fn join_texts(events: Vec<XmlEvent>) -> Vec<XmlEvent> {
    let mut joined: Vec<XmlEvent> = Vec::new();
    for event in events {
        match (joined.last_mut(), event) {
            (Some(XmlEvent::Text(previous)), XmlEvent::Text(next)) => previous.push_str(&next),
            (_, event) => joined.push(event),
        }
    }
    joined
}

fn expect_events(xml: &dyn XmlSyntax, text: &str, expected: &[XmlEvent]) -> Result<(), String> {
    let events = join_texts(read(xml, text)?);
    expect_equal(text, &events.as_slice(), &expected)
}

fn read(xml: &dyn XmlSyntax, text: &str) -> Result<Vec<XmlEvent>, String> {
    xml.read(text)
        .map_err(|error| format!("{text:?} gave {error:?}"))
}

fn expect_equal<T: PartialEq + std::fmt::Debug + ?Sized>(
    input: &str,
    actual: &T,
    expected: &T,
) -> Result<(), String> {
    if actual == expected {
        return Ok(());
    }
    Err(format!("{input:?} gave {actual:?}, not {expected:?}"))
}

fn start(name: XmlName) -> XmlEvent {
    XmlEvent::Start(XmlStart {
        name,
        ..XmlStart::default()
    })
}

fn plain(local: &str) -> XmlName {
    XmlName::new("", "", local)
}

fn attribute(name: XmlName, value: &str) -> XmlAttribute {
    XmlAttribute {
        name,
        value: value.to_owned(),
    }
}
