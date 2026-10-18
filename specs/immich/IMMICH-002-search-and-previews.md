# IMMICH-002 – Content search and previews from Immich

- Status: draft
- Module: immich (contributes a `SearchProvider` and a `PreviewDecoder`)
- Origin: IM6, IM7, IM9, B21
- Depends on: IMMICH-001 (connection, matching)

## Purpose

Immich already has CLIP embeddings and previews for the whole archive. Framehive uses
them live: content search ("dog on the beach") without a local ML stack, and previews
for files that are not synced to this machine.

## Behaviour

**Content search.** The search field offers "Content (Immich)" as a search kind. The
query is sent to Immich smart search. Results are mapped to local assets (IMMICH-001
matching) and shown in Immich's relevance order. Unmatched results are counted, but
not shown.

**Previews.** The `PreviewDecoder` chain tries, in order:

1. The local preview cache.
2. Embedded JPEG or decoding, only for `local` files.
3. The Immich thumbnail for matched assets, only while Immich is reachable.

Previews fetched from Immich are stored in the local cache, so they stay available
offline.

**Unavailability.** If Immich is unreachable, the search kind is shown as unavailable
with a reason, and the preview chain skips Immich.

## Acceptance criteria

### IMMICH-002.1 – Search results are local assets
- **Given** a fake Immich that returns assets A, B and C for "dog on the beach", where
  A and C are matched locally
- **When** the user searches for content "dog on the beach"
- **Then** the grid shows A and C in that order, plus the notice "1 result not in
  this library"

### IMMICH-002.2 – Preview for an online-only file
- **Given** an online-only, matched asset without a cached preview
- **When** the grid shows it
- **Then** the Immich thumbnail is shown, cached, and the local file is not read

### IMMICH-002.3 – Cached preview offline
- **Given** a preview fetched earlier from Immich
- **When** Immich is unreachable and the asset is shown
- **Then** the cached preview is shown

### IMMICH-002.4 – Search unavailable
- **Given** Immich is unreachable
- **When** the user opens the search kinds
- **Then** "Content (Immich)" is disabled with the reason "Immich unreachable"

## Out of scope

- "Find similar images" through Immich: later, if Immich offers a stable endpoint.
- Local CLIP embeddings: later provider (B21).

## Open questions

–

## History

| Date | Change |
|---|---|
| 2026-10-17 | Initial draft |
