# Framehive XMP namespace

Framehive writes metadata into standard XMP properties (architecture §8). Its own
namespace holds only what no standard covers: today, the records with which a sync
remembers which values it wrote, so it never overwrites a value the user changed
(ADR 0013, IMMICH-001). The design and its alternatives are in ADR 0041.

| | |
|---|---|
| Namespace URI | `https://polandy.github.io/framehive/ns/1.0/` |
| Preferred prefix | `framehive` |
| Written by | The Immich sync (IMMICH-001) |

The URI is an identifier and never has to resolve. As in every XMP namespace, the
prefix is only a preference: a reader matches the URI, never the prefix (ISO 16684-1).

## Versioning

Version 1.0 may gain new optional fields and new values of `framehive:Source`. A reader
keeps fields and values it does not know, unchanged, as it does for every unknown XMP
field. A change that an older reader would misread gets a new URI (`…/ns/2.0/`).

## Properties

### `framehive:SyncRecords`

Unordered array (`rdf:Bag`) of `SyncRecord` structures. Each entry says that one value
in the same XMP packet was written by a sync and still belongs to it. A value without a
matching record belongs to the user. When the array becomes empty, the property is
removed.

### `SyncRecord` structure

All fields are in the Framehive namespace and required.

| Field | Value type | Meaning |
|---|---|---|
| `framehive:Source` | Closed choice of Text: `immich` | The sync that wrote the value |
| `framehive:PropertyNamespace` | URI | Namespace URI of the top-level property that holds the value |
| `framehive:PropertyName` | Text | Local name of that property |
| `framehive:ValueHash` | Text | `blake3:` and 64 lowercase hexadecimal digits: the BLAKE3 hash of the value's canonical form (below) |

The property is named by URI and local name, not by a prefixed path, because a program
that rewrites the packet may choose other prefixes.

One record covers one *owned unit*: the whole value of a simple property, or one item
of an array. A packet never holds two records with the same source, property and hash.

## Owned units and canonical forms

The hash is computed over the UTF-8 bytes of the canonical form. The text is taken
exactly as stored, after XML unescaping, with no Unicode normalization, trimming or
number reformatting.

| Property form | Owned unit | Canonical form |
|---|---|---|
| Simple value (`xmp:Rating`, `photoshop:City`, …) | The value | The value text |
| Language alternative (`dc:description`) | The `x-default` item | The item text, without the `xml:lang` qualifier |
| Array of simple values (`dc:subject`, `lr:hierarchicalSubject`) | One item | The item text |
| Array of structures | One item | The structure's canonical form |
| `mwg-rs:Regions` | One item of its `mwg-rs:RegionList` | That item's canonical form as a structure |

**Canonical form of a structure.** For every simple value inside the structure, also
inside nested structures, write one entry:

    <path> U+0000 <value> U+0000

`<path>` names the fields from the outer structure inward, separated by one space. Each
step is the field's namespace URI immediately followed by its local name, the field's
RDF property IRI. Entries are sorted by their path, byte by byte. Neither U+0000 nor a
space can occur in an IRI, and U+0000 cannot occur in XML, so the form is unambiguous.
Qualifiers are not part of the form. A structure that contains an array has no
canonical form in version 1.0; a writer that needs one extends this document first.

For example, a face region with a name contributes the entries for
`http://www.metadataworkinggroup.com/schemas/regions/Area
http://ns.adobe.com/xmp/sType/Area#h` through `…/Area#y`, `…/regions/Name` and
`…/regions/Type`, sorted.

Because the text is hashed exactly, a program that only reformats a value, such as
`0.5` to `0.500000`, ends the sync's ownership of it. The sync then keeps the value and
lists it in its report, so the error is always on the safe side.

## How a sync uses the records

For each owned unit it would write, the sync looks for a record with its source and the
property:

- A record exists and a unit with that hash is still present: the user did not change
  it. The sync may update or remove the unit and its record.
- A record exists but no unit has that hash: the user changed or removed it. The sync
  deletes the record and leaves the property alone; ownership has ended.
- No record, property empty or item absent: the sync may write the unit and adds a
  record.
- No record, but a value is present: it belongs to the user and is kept.

The records live in the same packet as the values they describe, whether that is a
sidecar or XMP embedded in a DNG (ADR 0015).

## Example

An asset with the Immich rating 3 and the tag `Holiday/Beach`, written by the sync:

```xml
<rdf:Description rdf:about=""
    xmlns:xmp="http://ns.adobe.com/xap/1.0/"
    xmlns:dc="http://purl.org/dc/elements/1.1/"
    xmlns:lr="http://ns.adobe.com/lightroom/1.0/"
    xmlns:framehive="https://polandy.github.io/framehive/ns/1.0/">
  <xmp:Rating>3</xmp:Rating>
  <dc:subject><rdf:Bag><rdf:li>Beach</rdf:li></rdf:Bag></dc:subject>
  <lr:hierarchicalSubject>
    <rdf:Bag><rdf:li>Holiday|Beach</rdf:li></rdf:Bag>
  </lr:hierarchicalSubject>
  <framehive:SyncRecords>
    <rdf:Bag>
      <rdf:li rdf:parseType="Resource">
        <framehive:Source>immich</framehive:Source>
        <framehive:PropertyNamespace>http://ns.adobe.com/xap/1.0/</framehive:PropertyNamespace>
        <framehive:PropertyName>Rating</framehive:PropertyName>
        <framehive:ValueHash>blake3:58d6fd3dc609068615d66b6a2616cce521e6bd49bbcd7854e3f2573b142c6637</framehive:ValueHash>
      </rdf:li>
      <rdf:li rdf:parseType="Resource">
        <framehive:Source>immich</framehive:Source>
        <framehive:PropertyNamespace>http://purl.org/dc/elements/1.1/</framehive:PropertyNamespace>
        <framehive:PropertyName>subject</framehive:PropertyName>
        <framehive:ValueHash>blake3:4b39cc225917ff34345771334d4a50c080afad5604c9adca0090d24978402e34</framehive:ValueHash>
      </rdf:li>
      <rdf:li rdf:parseType="Resource">
        <framehive:Source>immich</framehive:Source>
        <framehive:PropertyNamespace>http://ns.adobe.com/lightroom/1.0/</framehive:PropertyNamespace>
        <framehive:PropertyName>hierarchicalSubject</framehive:PropertyName>
        <framehive:ValueHash>blake3:01dca971b92fd0d3a45343f5ee3628767298456232f961077492e3974246caaf</framehive:ValueHash>
      </rdf:li>
    </rdf:Bag>
  </framehive:SyncRecords>
</rdf:Description>
```

If the user then sets the rating to 5 in darktable, no unit of `xmp:Rating` has the
recorded hash any more, so the next sync drops that record and leaves the 5 alone.
