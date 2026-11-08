# CORE-009 – XMP sidecars

- Status: accepted
- Module: core (metadata service)
- Origin: N16, 5.1, M2, T11
- Depends on: –

## Purpose

Framehive keeps metadata in XMP sidecars next to the photos (ADR 0003). Other programs
write the same sidecars, so Framehive must read every form they use, change only the
values it means to change, and never lose a field it does not know.

## Behaviour

**Reading.** A sidecar is parsed into the XMP data model of ISO 16684-1: simple values,
structs, arrays (`rdf:Bag`, `rdf:Seq`, `rdf:Alt`), language alternatives and
qualifiers, each property identified by its namespace URI and name. Every RDF form the
standard allows is read (ADR 0039): property attributes, nested `rdf:Description`,
`rdf:parseType="Resource"`, `rdf:value` qualifiers, several `rdf:Description`
elements, an `<?xpacket?>` wrapper, and an `x:xmpmeta` or bare `rdf:RDF` root. A UTF-8
byte order mark is skipped.

A sidecar is refused, with an error naming the file and the reason, when it is not
well-formed XML, is not UTF-8, contains a document type declaration, or gives one
property two different values.

**Sidecar names.** Two namings exist (ADR 0040): `original_with_extension`
(`DSCF0042.RAF.xmp`, the default) and `original_without_extension` (`DSCF0042.xmp`).
Reading accepts both; the extension `xmp` is matched in any case.

**Merging.** A change sets a simple value, adds items to an unordered array, replaces
an ordered array, or sets one language of a language alternative. Every other
property, qualifier and value text stays exactly as read.

**Writing.** The packet is serialized in Framehive's canonical form (ADR 0039):

- UTF-8 without a byte order mark, LF line ends, a final newline;
- an `x:xmpmeta` root with one `rdf:RDF` and one `rdf:Description rdf:about=""`; no
  `<?xpacket?>` wrapper, no padding;
- simple values as elements, structs with `rdf:parseType="Resource"`, language as
  `xml:lang`, other qualifiers in the `rdf:value` form;
- namespace declarations and properties ordered by prefix, then by property name;
  array items in their order;
- the preferred prefix for known namespaces (`dc`, `lr`, `mwg-kw`, `photoshop`, `xmp`,
  `framehive`, …), the file's own prefix for any other one;
- two spaces of indent per level.

Every write sets `xmp:MetadataDate` to the current time from the `Clock` port, with
seconds and the local UTC offset. It never sets `xmp:CreatorTool`, `xmp:ModifyDate` or
`xmpMM:History`.

**Saving.** A sidecar is written through the `files` port to `<name>.part` and then
renamed. Before the rename, the writer compares the file at the target with what it
found there before: if a sidecar appeared or its size, modification time or BLAKE3
changed, the target is left as it is and the write fails. A sidecar that cannot be
parsed is never overwritten. Permissions are never changed.

## Acceptance criteria

### CORE-009.1 – Every RDF form gives the same model
- **Given** fixtures that state `dc:title`, `dc:subject`, `xmp:Rating` and a struct
  once as attributes, once as elements, once with nested `rdf:Description`, once with
  `rdf:parseType="Resource"` and once split over two `rdf:Description` elements
- **When** each is parsed
- **Then** all give the same model

### CORE-009.2 – Wrapper and root forms
- **Given** one packet with an `<?xpacket?>` wrapper and padding, the same inside
  `x:xmpmeta` without a wrapper, and the same as a bare `rdf:RDF` root with a UTF-8 byte
  order mark
- **When** each is parsed
- **Then** all give the same model

### CORE-009.3 – Unknown fields survive a merge
- **Given** a sidecar with properties of an unknown namespace, a struct with a
  qualifier and a language alternative with `de` and `x-default`
- **When** `photoshop:TransmissionReference` is set to `Lisbon` and the sidecar is
  written
- **Then** reading it again gives the original model plus that value and the new
  `xmp:MetadataDate`, with every other value text unchanged

### CORE-009.4 – Canonical form
- **Given** the model of CORE-009.1 and the clock at `2026-11-08T14:03:12+01:00`
- **When** it is serialized
- **Then** the bytes equal the checked-in fixture `canonical.xmp`, with no
  `<?xpacket?>` wrapper, properties ordered by prefix and name, and
  `xmp:MetadataDate` `2026-11-08T14:03:12+01:00`

### CORE-009.5 – Serializing is deterministic
- **Given** a model built by setting the same properties in two different orders
- **When** both are serialized
- **Then** the bytes are equal

### CORE-009.6 – Prefixes
- **Given** a sidecar that declares `http://purl.org/dc/elements/1.1/` as `d` and an
  unknown namespace as `acme`
- **When** it is written again
- **Then** the Dublin Core properties use `dc`, and the unknown ones keep `acme`

### CORE-009.7 – A clashing prefix
- **Given** a sidecar that declares an unknown namespace with the prefix `dc`
- **When** it is written again
- **Then** Dublin Core keeps `dc`, the unknown namespace gets `dc1`, and reading it
  again gives the same model

### CORE-009.8 – Unordered arrays are unioned
- **Given** a sidecar with `dc:subject` `Beach`, `Sea`
- **When** `Sea` and `Sun` are added
- **Then** `dc:subject` is `Beach`, `Sea`, `Sun`

### CORE-009.9 – One language is set
- **Given** a sidecar with `dc:rights` `de` `© Andy` and `x-default` `© A.`
- **When** `x-default` is set to `© 2026 Andy Pollari`
- **Then** `de` is unchanged and `x-default` is `© 2026 Andy Pollari`

### CORE-009.10 – Housekeeping properties
- **Given** a sidecar with `xmp:CreatorTool` `darktable` and no `xmp:MetadataDate`
- **When** it is written with the clock at `2026-11-08T14:03:12+01:00`
- **Then** `xmp:MetadataDate` is `2026-11-08T14:03:12+01:00`, `xmp:CreatorTool` is
  still `darktable`, and no `xmp:ModifyDate` or `xmpMM:History` is added

### CORE-009.11 – Malformed XML is refused
- **Given** a sidecar whose `rdf:Description` is not closed
- **When** it is read
- **Then** an error names the file and the position, and nothing is written

### CORE-009.12 – A document type declaration is refused
- **Given** a sidecar with a `<!DOCTYPE>` that declares an entity
- **When** it is read
- **Then** an error names the file and the reason, and the entity is never expanded

### CORE-009.13 – Encodings other than UTF-8 are refused
- **Given** a sidecar in UTF-16
- **When** it is read
- **Then** an error names the file and says it is not UTF-8

### CORE-009.14 – Conflicting values are refused
- **Given** a sidecar whose two `rdf:Description` elements give `xmp:Rating` `2` and `4`
- **When** it is read
- **Then** an error names the file and the property

### CORE-009.15 – A sidecar that cannot be parsed is kept
- **Given** a target sidecar that cannot be parsed
- **When** a value is to be merged into it
- **Then** the write fails with the parse error, and the file is byte for byte as before

### CORE-009.16 – The write is atomic
- **Given** a sidecar being written, and the `files` port failing the rename
- **When** the write ends
- **Then** the target is as before, no `.part` file is left, and the error names the
  path

### CORE-009.17 – A sidecar that appeared is not replaced
- **Given** no sidecar at the target when the write began, and another program
  creating one before the rename
- **When** the write reaches the rename
- **Then** the other program's sidecar is unchanged, and the write fails with an error
  that names the path

### CORE-009.18 – A sidecar changed meanwhile is not replaced
- **Given** a sidecar read for a merge, and its content changed by another program
  before the rename
- **When** the write reaches the rename
- **Then** the changed sidecar is kept, and the write fails with an error that names
  the path

### CORE-009.19 – A read-only sidecar
- **Given** a target sidecar the user cannot write
- **When** a value is merged into it
- **Then** the write fails with an error that names the path, and the file's
  permissions and content are unchanged

### CORE-009.20 – Both sidecar names are read
- **Given** `DSCF0042.RAF` with `DSCF0042.RAF.XMP` next to it, and `DSCF0043.RAF` with
  `DSCF0043.xmp`
- **When** the sidecars of each file are looked up
- **Then** `DSCF0042.RAF.XMP` is found for the first and `DSCF0043.xmp` for the second

### CORE-009.21 – Names follow the naming setting
- **Given** the file `20261108-0042.RAF`
- **When** its sidecar name is made with `original_with_extension` and with
  `original_without_extension`
- **Then** the names are `20261108-0042.RAF.xmp` and `20261108-0042.xmp`

### CORE-009.22 – exiftool reads what Framehive writes
- **Given** a sidecar written with job code, creator, copyright and the keyword
  `Holiday/Beach`
- **When** exiftool reads it
- **Then** it reports the same values and no warning

### CORE-009.23 – Sidecars of other programs survive
- **Given** the checked-in sidecars exported from darktable and digiKam
- **When** a keyword is merged into each and it is written
- **Then** reading it again gives every original property, including darktable's
  history and digiKam's `TagsList`, unchanged

## Out of scope

- Which values the import writes, and when: IMPORT-019 and IMPORT-020.
- Embedded XMP in DNG (ADR 0015, CORE-004) and in JPEG or HEIF (5.1).
- Detecting external changes (S6), merging between machines (S8, T13) and Nextcloud
  conflict copies (NC4): the library.
- Rating, pick state and color labels: library editing.

## Open questions

–

## History

| Date | Change |
|---|---|
| 2026-11-08 | Created and accepted with the owner's XMP decisions D7–D10, D24, D25 (ADR 0039, ADR 0040) |
| 2026-11-08 | CORE-009.1–.14 implemented: XMP model, parser and canonical serializer in `metadata`, `adapter-xmp` (ADR 0039) |
