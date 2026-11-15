# 0039 – XMP parser and serializer: quick-xml and an own model

- Status: accepted
- Date: 2026-11-08

## Context

The XMP milestone makes Framehive write XMP for the first time: the import writes job
code, creator, copyright and keywords (N16) to sidecars (ADR 0040). Every later writer
(library edits B8/B9, the Immich sync IMMICH-001, grouping LIBRARY-001) reads and merges
the same packets, so the model must be right from the start (owner decision D1).

ADR 0009 lists the Adobe XMP Toolkit for XMP, with "our own reader/writer limited to the
scope we need" as the alternative. ADR 0003 and CLAUDE.md rule 3 require that unknown
namespaces and fields are never dropped. Sidecars come from other programs, cameras and
phones, so the parser reads untrusted input.

The options the owner weighed (D7):

- `quick-xml` for XML tokens, with our own XMP model, parser and serializer.
- Entirely our own code, including the XML tokenizer.
- The Adobe XMP Toolkit through `xmp_toolkit`: native C++ with its own Nix build on
  Linux and macOS.
- exiv2 through FFI: native C++, GPL-2.0-or-later, a history of CVEs, and a second
  metadata stack next to the own EXIF reader (ADR 0021).

## Decision

### 1. quick-xml tokenizes, our own code models XMP (D7)

`quick-xml` (pure Rust, MIT) reads and writes XML tokens. It is wrapped in exactly one
adapter crate, `adapter-xmp`, which turns tokens into a small XML event type of our own
and back. The XMP data model of ISO 16684-1 (simple values, structs, arrays,
qualifiers, language alternatives), the RDF parser and the serializer are our own code
in the `metadata` service, so the tokenizer stays replaceable. The dependency gets its
`dependencies.yaml` entry and passes `cargo deny` with the PR that adds it.

The parser accepts every RDF form ISO 16684-1 allows: property attributes, nested
`rdf:Description`, `rdf:parseType="Resource"`, `rdf:value` qualifiers, several
`rdf:Description` elements, an `<?xpacket?>` wrapper and an `x:xmpmeta` or bare
`rdf:RDF` root. It refuses document type declarations and entities, input that is not
UTF-8, and two different values for one property. It gets a fuzz target in
`metadata-fuzz` (ADR 0037) and a deterministic mutation test.

### 2. A rewrite keeps every value, not the bytes (D8)

A merged packet is serialized again from the parsed model. Every property, qualifier
and value text is kept exactly; layout, prefixes and the attribute or element form
become Framehive's canonical form. ISO 16684-1 gives prefixes and layout no meaning,
and sync-record hashes (ADR 0041) stay valid because value text is unchanged. A packet
that cannot be parsed is never rewritten.

### 3. The canonical form (D9)

- UTF-8 without a byte order mark, LF line ends, a final newline.
- An `x:xmpmeta` root holding one `rdf:RDF` with one `rdf:Description rdf:about=""`;
  no `<?xpacket?>` wrapper and no padding, which XMP Part 3 makes optional in sidecars.
- Simple values as elements, structs as `rdf:parseType="Resource"`, arrays as
  `rdf:Bag`, `rdf:Seq` or `rdf:Alt`, language as `xml:lang`, other qualifiers in the
  `rdf:value` form.
- Namespaces and properties in a fixed order: by prefix, then by property name; array
  items keep their order.
- The preferred prefix for known namespaces, the file's own prefix for any other one.
- Two spaces of indent per level.

### 4. Housekeeping (D10)

Each write sets `xmp:MetadataDate` to the current time from the `Clock` port. ADR 0015
and CORE-004 resolve conflicts by the newer `xmp:MetadataDate`, so it must always be
set. `xmp:CreatorTool` and `xmp:ModifyDate` describe the image, not the metadata, and
are never set; no `xmpMM:History` entry is added.

## Consequences

- One pure-Rust dependency, no native code; XML tokenizing, the security-relevant part
  of untrusted input, is left to a widely used library.
- Every RDF form must be covered by our own tests: checked-in text fixtures for each
  form (ADR 0017 allows text), conformance tests from the specification examples, and
  round trips through exiftool (CORE-009, D25).
- Other programs see a full rewrite of a sidecar the first time Framehive merges into
  it; later rewrites differ only where values changed.
- The library, the Immich sync and DNG embedding (ADR 0015) reuse the model without a
  second XMP stack.
- ADR 0009's table entry for XMP now points here.

## Alternatives

- **Entirely own code (D7 B).** No dependency, but XML (entities, CDATA, namespaces,
  encodings) is security-relevant parsing of untrusted input where a vetted tokenizer
  pays off.
- **Adobe XMP Toolkit (D7 C).** Reference behaviour, but native C++ with an own Nix
  build on two platforms, an ADR for native code and a large surface for a small need.
- **exiv2 (D7 D).** Also handles embedded XMP and IPTC-IIM, but native C++, a CVE
  history and a second metadata stack.
- **Byte-preserving edits (D8 B).** Smallest diffs for other programs and Nextcloud, but
  fragile across the many RDF forms.
- **A backup file before each rewrite (D8 C).** Safer for early releases, but leaves
  `.xmp.bak` files the library must ignore.
- **The `<?xpacket?>` wrapper (D9 B) or mimicking the existing form (D9 C).** Not needed
  in sidecars; mimicking costs code for friendlier diffs only.
- **An `xmpMM:History` entry (D10 B) or no housekeeping (D10 C).** History grows each
  sidecar and is rarely shown; without `xmp:MetadataDate` ADR 0015 has nothing to compare.
