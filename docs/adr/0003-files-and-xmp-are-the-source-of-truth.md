# 0003 – Files and XMP are the source of truth, one index per machine

- Status: accepted
- Date: 2026-10-17

## Context

Libraries live locally, on a NAS and on external drives that are sometimes offline;
several machines (Linux + Mac) access them. No lock-in to darktable, digiKam or
Lightroom.

## Decision

- Metadata is stored in standards-compliant XMP sidecars (see `docs/architecture/`
  section 8). These are the single source of truth.
- Each machine has its own SQLite index. It never lives on a network drive and can be
  rebuilt from the files at any time.
- Data without an XMP equivalent (albums, saved searches) is stored as versioned YAML
  files under `<library root>/.framehive/`.
- Library roots are identified by a UUID in a marker file; paths are relative to the
  library root.

## Consequences

- Exchange with other programs and between machines works without a server.
- Writing XMP must be robust (atomic, preserving unknown fields, merging on
  conflicts); this needs its own testing focus (T11, T13).
- Compute-intensive data (embeddings, hashes) is a cache and is computed per machine.
