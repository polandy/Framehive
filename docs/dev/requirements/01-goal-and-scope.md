# 1. Goal & Scope

Part of the [requirements](README.md).

Framehive is a photo library made up of modules that share a common data store:

| Module | Purpose | Phase |
|---|---|---|
| **Import** | Fetch photos and videos from cameras, cards, smartphones, and folders; rename, store, back up in parallel, and verify them. In the end, you should **know for certain that the card can be formatted**. Runs in the background (4.6) | MVP |
| **Library** | Browsing, viewing, rating, tagging, culling, searching, albums (section 5) | MVP |
| **Map** | Images by location, geotagging | soon after v1 |
| **People** | Face naming and person filter; faces from Immich in v1, local recognition soon after v1 | v1 |
| Editor | Image editing | later; until then, hand off to darktable, RawTherapee, GIMP |

Guiding principle: **The files and their XMP sidecars are the source of truth**; the
database is only an index that can be rebuilt at any time. No lock-in; darktable,
digiKam, and Lightroom see the same tags and ratings.

Out of scope (for now): image editing, a proprietary cloud service, Windows.
