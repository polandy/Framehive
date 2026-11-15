# 0044 – Rejected pick state as `xmp:Rating` -1

- Status: accepted
- Date: 2026-11-10

## Context

Remove RAWs skips rejected assets (LIBRARY-002, B30), and the library reads every
asset's state back from its sidecars, so an index rebuild gives the same result
(LIBRARY-001). The library therefore has to read the rejected pick state from XMP
before the pick state as a whole (B6) is built.

No standard defines a pick state (architecture §8). The XMP specification gives
`xmp:Rating` the values -1 (rejected), 0 (unrated) and 1 to 5. Lightroom, darktable and
digiKam write a rejected photo as `xmp:Rating` -1. digiKam also writes its own
`digiKam:PickLabel`, which other programs do not read.

## Decision

1. An asset is **rejected** when `xmp:Rating` is a simple value that reads as a number
   below zero, such as `-1` or `-1.0`. A missing rating, `0` to `5`, text that is not a
   number, and a struct or array are not rejected.
2. `metadata` reads it as `XmpPacket::is_rejected`. When an asset's sidecars disagree,
   the sidecar that holds its other state decides (LIBRARY-001).
3. Writing a rejected state writes `xmp:Rating` -1 and replaces the stars. This comes
   with the pick state (B6); its *keep* and *pending* values, which have no common XMP
   form, need their own decision then.

## Alternatives

- **`digiKam:PickLabel`:** carries all three pick values, but only digiKam reads it.
  The digiKam migration (M1) can map it to `xmp:Rating` -1.
- **A property in the Framehive namespace:** no other program would see a rejected
  photo, and Lightroom's and darktable's rejects would be lost.
- **Only the text `-1`:** misses `-1.0`, which a program that writes the rating as a
  real number may produce.

## Consequences

- Rejects from Lightroom, darktable and digiKam are skipped by Remove RAWs.
- A rejected asset has no star rating of its own; un-rejecting it leaves it unrated.
