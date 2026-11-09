<!--
SPDX-FileCopyrightText: 2026 Framehive contributors
SPDX-License-Identifier: GPL-3.0-or-later
-->

# metadata

Layer: `service` (architecture §2, §3)

## Purpose

The metadata service reads metadata from photo files. It reads the capture time
and the camera model (`Model`, IMPORT-021) from EXIF with its own parser (CORE-005,
ADR 0021). It reads only the few
kilobytes that lead to the EXIF, never a whole RAW file.

It reads and writes XMP packets (CORE-009, ADR 0039) in `xmp`: the data model of
ISO 16684-1, a parser for every RDF form and a serializer for Framehive's canonical
form. XML tokens come through the `XmlSyntax` port, which `adapter-xmp` implements.

- **Model.** A packet maps property names (namespace and local name) to nodes; a node
  is a text, a URI, a struct or a Bag, Seq or Alt array, with qualifiers such as
  `xml:lang`. Packets compare by their properties only.
- **Parsing** accepts property attributes, nested `rdf:Description`,
  `rdf:parseType="Resource"`, `rdf:value` qualifiers, several `rdf:Description`
  elements, an `<?xpacket?>` wrapper, an `x:xmpmeta` or `x:xapmeta` root or a bare
  `rdf:RDF` root, and a UTF-8 byte order mark. It refuses input that is not UTF-8, a
  document type declaration, malformed XML (with line and column) and two different
  values for one property. Every error names the file.
- **Editing** sets a simple value (keeping its qualifiers), unions a Bag, replaces a
  Seq and sets one language of an Alt; everything else stays as read.
- **Keep RAW** (LIBRARY-002, ADR 0042) reads and writes `framehive:KeepRaw`: marking
  writes `True`, unmarking removes it, and only `True` in any case reads as marked.
- **Serializing** writes `x:xmpmeta`, `rdf:RDF` and one `rdf:Description`, with the
  namespaces declared on it and sorted by prefix, the properties sorted by prefix and
  name, two-space indent, LF and a final newline, no `<?xpacket?>`. Simple values are
  elements, structs use `rdf:parseType="Resource"`, other qualifiers the `rdf:value`
  form. Known namespaces get their preferred prefix (`xmp::namespaces`); an unknown one
  keeps the file's prefix, with a number when it clashes (`dc1`). `xmp:MetadataDate`
  is set from the `Clock` port in local time with offset; `xmp:ModifyDate`,
  `xmp:CreatorTool` and `xmpMM:History` are never touched.
- **Sidecar files** (ADR 0040 §1, §7). `SidecarNaming` makes a sidecar's name in
  either naming; `find_sidecars` lists the sidecars of a file in both, with the
  extension in any case. `SidecarFiles` reads, merges and saves them through the
  `FileSystem` port: the content goes to `<name>.part`, is synced, and replaces the
  sidecar only if the sidecar's size, modification time and checksum (BLAKE3 in the
  apps, from the `Hasher` port) are still what was read, or nothing appeared where
  there was none. A new sidecar is renamed with `rename_no_replace`, an existing one
  with `replace_file`, which keeps its permissions and refuses a read-only file. A
  sidecar that cannot be parsed is never overwritten, and a failed write removes its
  `.part` file.

## Public API

| Item | Meaning |
|---|---|
| `read_capture_time` | The capture time of a file, read through the `ReadOnlyFileSystem` port; formats not in the CORE-005 table are not opened |
| `CaptureTimeError` | Why `read_capture_time` gives no capture time: a `NoCaptureTime` reason or an I/O error with the path |
| `capture_time_from_bytes` | The capture time of a file's content in memory |
| `NoCaptureTime` | Why a file has no capture time |
| `xmp::XmpPacket`, `XmpName`, `XmpNode`, `XmpValue`, `ArrayKind` | The XMP data model; `XmpPacket::set_text`, `add_to_bag`, `replace_seq`, `set_language`, `add_keywords` and `set_keep_raw` (hierarchical keywords in `dc:subject`, `lr:hierarchicalSubject` and `mwg-kw:Keywords`, ADR 0040) edit it; `keep_raw` reads the Keep RAW marker |
| `xmp::parse_xmp` | Parses a sidecar's bytes into an `XmpPacket` |
| `xmp::serialize_xmp` | Writes an `XmpPacket` in the canonical form, setting `xmp:MetadataDate` |
| `xmp::XmpError`, `XmpErrorReason` | Why a sidecar cannot be read, with its path |
| `xmp::XmpEditError`, `ClockOutOfRange` | Why an edit or a write is refused |
| `xmp::SidecarNaming`, `find_sidecars` | A sidecar's name in either naming, and the sidecars next to a file |
| `xmp::SidecarFiles`, `ReadSidecar`, `FoundSidecar` | Reads, merges and atomically saves sidecar files through the `FileSystem` and `Hasher` ports, refusing to replace a sidecar another program created or changed meanwhile |
| `xmp::SidecarError`, `SidecarErrorKind`, `SidecarStep` | Why a sidecar file could not be read or saved, with its path |
| `xmp::XmlSyntax`, `XmlEvent`, `XmlStart`, `XmlName`, `XmlAttribute`, `XmlSyntaxError`, `XmlSyntaxErrorKind` | The port that reads and writes XML tokens |
| `xmp::namespaces` | The namespace URIs used by name, and the preferred prefix of each known namespace |
| `testing::Tiff`, `testing::ByteOrder`, `testing::jpeg`, `testing::raf` | Builders for test files with EXIF text tags |
| `testing::check_xml_syntax` | The conformance suite of the `XmlSyntax` port |
| `testing::RecordedXml` | An `XmlSyntax` double that records the events it is asked to write |

## Dependencies

- `domain`
- `files` (the `ReadOnlyFileSystem`, `FileSystem` and `Hasher` ports)
- `thiserror` (ADR 0016)
