<!--
SPDX-FileCopyrightText: 2026 Framehive contributors
SPDX-License-Identifier: GPL-3.0-or-later
-->

# adapter-xmp

Layer: `adapter`, port `metadata` (architecture §3)

## Purpose

Implements `metadata::xmp::XmlSyntax` with `quick-xml` (ADR 0039, decision 1). It turns
text into the port's XML events and events back into text. The XMP model, the RDF
parser and the canonical serializer stay in `metadata`, so no other crate sees a
`quick-xml` type.

Reading:

- resolves every element and attribute name to its namespace, from the declarations
  in scope; an unprefixed attribute is in no namespace, `xml` is predefined;
- refuses a name that is not an XML name, an undeclared prefix, `<` in an attribute
  value and an unclosed or mismatched element;
- refuses a document type declaration, so no entity is ever declared or expanded; of
  the references, only the five predefined entities and character references exist;
- normalizes attribute values and line ends as XML 1.0 asks, joins CDATA into text
  and skips the XML declaration, comments and processing instructions such as
  `<?xpacket?>`.

Errors carry the byte offset; `metadata` turns it into a line and column.

Writing escapes text and attribute values so that they read back unchanged, and adds
no whitespace of its own. It is a small string writer; `quick-xml` only escapes.

## Public API

| Item | Meaning |
|---|---|
| `QuickXml` | `metadata::xmp::XmlSyntax` with `quick-xml`; `QuickXml::new()` or `Default` |

## Tests

`tests/conformance.rs` runs `metadata::testing::check_xml_syntax`. `tests/reading.rs`
and `tests/writing.rs` test CORE-009.1–.14 end to end, `parse_xmp` and
`serialize_xmp` with this adapter, on the sidecars in `tests/fixtures/`.
`canonical.xmp` is the canonical form byte for byte. UTF-16 and byte-order-mark inputs
are built in code. `tests/mutated_packets.rs` damages the fixtures with a seeded PRNG:
no input may panic, and every accepted one must read back the same after it is
written. The fuzz target `xmp` in `fuzz/` does the same with libFuzzer (ADR 0037).
`tests/exiftool.rs` runs the round trips with exiftool (CORE-009.22, .24, D25):
exiftool reads what Framehive writes, and Framehive reads, merges into and writes back
what exiftool wrote, compared field by field in exiftool's `-struct` output. It calls
the `exiftool` of the Nix shell over standard input and output and checks that it is the
version `flake.lock` pins (13.59); a `nixpkgs` update that changes it must update the
test.

## Dependencies

- `metadata`
- `quick-xml` (ADR 0039): version 0.42, no features
