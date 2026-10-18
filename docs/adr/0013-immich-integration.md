# 0013 – Immich integration (read-only)

- Status: proposed
- Date: 2026-10-17

## Context

The project owner's long-term archive is a self-hosted setup:

- **Nextcloud** (Docker) stores the photos. Framehive machines (Linux PC, MacBook) see
  them through the Nextcloud desktop client's sync folder. Files there are sometimes
  synced locally and sometimes online-only.
- **Immich** (Docker) indexes the same Nextcloud data volume as an *external library*,
  mounted **read-only** into the container. Immich has already done face recognition,
  person naming, CLIP embeddings, reverse geocoding, albums, tags and favorites.

That work should not be redone, and Framehive must not depend on Immich being
reachable, for example when travelling. Immich must never be written to.

## Decision

1. **New module `immich`** (crates `immich` + `immich-ui`) with its own settings and
   jobs. It talks to the kernel only through events, commands and services, like any
   module (ADR 0004). The port `ImmichApi` lives in the module. The adapter
   `adapter-immich-http` implements it.
2. **Strictly read-only.** The adapter only sends `GET` requests and the read-only
   search endpoints (`POST /api/search/metadata`, `POST /api/search/smart`), with an
   API key. Setup asks for a key with read permissions only. A conformance test checks
   that the adapter cannot issue any other request.
3. **Data flows one way, into standard XMP.** A sync job writes Immich data into
   Framehive's sidecars, so files stay the source of truth (ADR 0003). The result is
   available offline and readable by digiKam, darktable and Lightroom.

   | Immich | XMP (standard) |
   |---|---|
   | Faces + person names | MWG regions (`mwg-rs:RegionList`, type `Face`, `mwg-rs:Name`) |
   | Tags | `lr:hierarchicalSubject` + `dc:subject` |
   | Favorite | Tag `Immich/Favorite` (name configurable) |
   | Rating | `xmp:Rating` |
   | Description | `dc:description` |
   | City, state, country | `photoshop:City`, `photoshop:State`, `photoshop:Country` |
   | Albums | Framehive albums (`.framehive/albums/*.yaml`) with `source: immich`, read-only in Framehive |

4. **Manual edits are never overwritten.** The sync records which values it wrote,
   with a hash, in a Framehive XMP namespace. That namespace is documented in
   `docs/xmp-namespace.md`; its URI must be one the project controls, and it is fixed
   before this ADR is accepted. A later sync updates or removes only values that
   still match what it wrote. A field the user changed belongs to the user from then
   on.
5. **Live features use Immich directly** while it is reachable:
   - Content search through the `SearchProvider` extension point, using Immich smart
     search. Results are mapped to local assets.
   - Previews for online-only files through the `PreviewDecoder` extension point,
     using Immich thumbnails.

   When Immich is unreachable, these providers report "unavailable". Nothing else
   changes.
6. **Matching Immich assets to local files**:
   1. Path mapping configured in YAML, for example container prefix
      `/data/nextcloud/andy/files/` → local `~/Nextcloud/`.
   2. Fallback: the SHA-1 checksum that Immich stores, compared with the local file.
      This is only tried for locally synced files, so it never downloads anything.
   3. Unmatched assets are listed and skipped.
7. **People module (B19)**: face data comes through a provider port. Immich is the
   first provider. Local recognition (ONNX) is added later as a second provider for
   photos that are not in Immich.
8. **Dependencies** (ADR 0009):
   - `ureq` with `rustls`: a synchronous HTTP client that fits the std-thread job
     scheduler, with established TLS.
   - `serde_json`.
   - `sha1` (RustCrypto).

   All of them are only in `adapter-immich-http`.
9. **Supported Immich versions** are a pinned range. The Immich API changes often.
   - Adapter tests run against recorded responses served by an in-process loopback
     server. No external network, so they are deterministic (ADR 0006).
   - A nightly contract test runs against the pinned Immich Docker image.
   - Outside the range, Framehive warns and disables the module instead of guessing.

## Alternatives

- **Read Immich's PostgreSQL database directly:** faster, but an undocumented internal
  schema, and it needs database access to the server. Rejected.
- **Live only, nothing stored:** no people or albums while offline, and other programs
  never see the data. Rejected by the project owner.
- **Writing back to Immich:** not wanted. Immich mounts the data read-only.

  Instead, Framehive's sidecars land in the Nextcloud folder, so Immich reads them on
  its next library scan. That is an indirect, standards-based channel with no write
  API access.

## Consequences

- People, search and places work early, without a local ML stack.
- One more module, three more dependencies, and a moving external API. This is
  contained by the port, the version range and the contract test.
- Risk: Immich may re-import face regions from Framehive's sidecars and create
  duplicates. This has to be verified with the pinned version before acceptance. If it
  happens, region writing becomes configurable.
