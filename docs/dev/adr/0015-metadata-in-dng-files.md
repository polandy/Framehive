# 0015 – Metadata in DNG files

- Status: accepted
- Date: 2026-10-18

## Context

Framehive writes metadata to XMP sidecars and never changes originals (ADR 0003,
requirements 5.1). DNG is a special case:

- Lightroom and Adobe Camera Raw read and write the XMP **embedded** in a DNG, and
  ignore a `.xmp` sidecar next to it. A user who also opens their DNGs in Lightroom
  would not see Framehive's ratings and tags.
- The DNG specification allows the XMP packet (TIFF tag 700) to be rewritten. The raw
  image digest (`NewRawImageDigest`, `RawImageDigest`) covers the image data only, so
  it lets us prove that rewriting did not touch the image.
- Changing an original carries risk: a broken write can destroy a RAW. Every change
  also makes the Nextcloud client upload the whole file again.

## Decision

1. **The sidecar is always written.** It stays the primary carrier of metadata, as for
   every other format.
2. **Embedding is an option**, `dng_metadata: sidecar_and_embedded`, off by default.
   When it is on, the same XMP packet is also written into the DNG.
3. **Embedding is safe by construction:** write to `<name>.part`, re-read it, compare
   the raw image digest (or, if the file has none, a hash of the raw image data we
   compute before and after), then rename atomically. On any failure the original is
   left untouched.
4. **Reading merges both** sources. Fields that exist in only one source are kept. If a
   field differs, the newer `xmp:MetadataDate` wins, and the asset shows the conflict
   badge (LIBRARY-001). Unknown fields from both are preserved.
5. **Only DNG** gets this option. Proprietary RAW formats (RAF, CR2, …) are never
   written to.

## Alternatives

- **Sidecar only:** the safest option, but Lightroom users lose their metadata on DNG.
- **Always embed (like Lightroom):** compatible, but changes originals by default and
  causes large uploads in the sync folder for every rating.
- **Embed only, no sidecar:** darktable and digiKam prefer sidecars, and a sidecar keeps
  working when the DNG is online-only (CORE-002).

## Consequences

- With the option on, the DNG's file hash changes after each metadata write. The index
  stores the new hash. Duplicate detection and import history must not rely on the
  whole-file hash of a DNG staying stable; they use the raw image digest where present.
- Immich sees a changed file and re-reads it. The path mapping still matches it
  (ADR 0013).
- Writing into an `online_only` DNG is deferred until the file is local. The sidecar is
  written immediately.
- Fault-injection tests cover the embedding path (requirements 9.2).
