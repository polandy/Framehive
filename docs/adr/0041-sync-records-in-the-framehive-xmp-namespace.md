# 0041 – Sync records in the Framehive XMP namespace

- Status: accepted
- Date: 2026-11-07
- Renumbered: from 0037 on 2026-11-08, because two ADRs had that number

## Context

ADR 0013 lets the Immich sync write into standard XMP properties but never overwrite a
value the user changed. For that, the sync records which values it wrote, with a hash,
in Framehive's own namespace `https://polandy.github.io/framehive/ns/1.0/`. No standard
defines such fields, so CLAUDE.md rule 3 asks for an ADR before the mapping is
implemented. ADR 0013 left the field names and the hash format open and points to
`docs/xmp-namespace.md`.

The records must survive other programs: digiKam, darktable and Lightroom keep unknown
fields but may rewrite a packet with other prefixes, reorder array items or reformat
values. IMMICH-001 compares tags and regions per entry, not per property.

## Decision

1. **One property, `framehive:SyncRecords`**, an `rdf:Bag` of `SyncRecord`
   structures in the same packet as the values they describe. A record names its
   `Source` (`immich`), the property by `PropertyNamespace` (URI) and `PropertyName`,
   and the `ValueHash` of one owned unit.
2. **An owned unit is a simple value, the `x-default` item of a language alternative,
   or one array item.** Tags and regions are thus owned per entry.
3. **The hash is BLAKE3 over a canonical form, written as `blake3:<hex>`**, the
   checksum format of architecture §8. Simple text is hashed exactly as stored. A
   structure is hashed as its sorted leaf entries `<path> U+0000 <value> U+0000`, with
   the path made of RDF property IRIs.
4. **Exact text, no normalization.** A value another program merely reformats counts as
   changed by the user, so the sync keeps it.
5. **Versioning.** Version 1.0 may gain optional fields and new sources; readers keep
   what they do not know. An incompatible change gets a new namespace URI.

The full definition, with an example, is `docs/xmp-namespace.md`.

## Alternatives

- **Store the written value instead of a hash:** simpler to debug, but it copies every
  description and person name into a second place where search tools and other
  programs index them, and a long description doubles in size. Rejected.
- **Name the property with a prefixed XMP path (`xmp:Rating`):** shorter, but prefixes
  are not significant in XMP (ISO 16684-1), and a program may rewrite them. Rejected.
- **One record per property with a hash over the whole array:** any tag the user adds
  would end the sync's ownership of all its tags, against IMMICH-001. Rejected.
- **Normalize numbers and Unicode before hashing:** fewer lost ownerships after another
  program rewrites a packet, but every normalization rule is one more chance to treat a
  user's change as unchanged and overwrite it. Rejected in favor of the safe side.
- **SHA-1 or SHA-256:** BLAKE3 is already the project's checksum (ADR 0020), so no new
  dependency is needed.

## Consequences

- The Immich sync can be implemented without further format decisions. It computes the
  hash through the existing hashing port (`files::Hasher`, implemented by
  `adapter-blake3`), so the module itself gains no dependency.
- A record that a sync no longer recognizes, for example one from a newer Framehive,
  stays in the packet unchanged.
- A packet rewritten by a program that reformats values loses sync ownership of those
  values; the sync report lists them, and the user's data is never overwritten.
