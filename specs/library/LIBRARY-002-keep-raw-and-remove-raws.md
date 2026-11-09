# LIBRARY-002 – Keep RAW and Remove RAWs

- Status: accepted
- Module: library
- Origin: B29, B30
- Depends on: LIBRARY-001

## Purpose

Many photographers shoot RAW + JPEG but want the RAW only for the few pictures they will
edit. The Keep RAW marker records which RAWs stay. Remove RAWs then frees the space of
all other RAWs in one step, without touching the JPEGs or any metadata.

## Behaviour

**Keep RAW marker.** Every asset has a Keep RAW marker, on or off (default off). It is
its own toggle, independent of rating, pick state and color label: a rejected asset can
be marked, and changing the rating does not change the marker. Key `R` toggles it for
the selected assets, in the grid and the loupe; the command palette has the same
command. `R` is free in the library key map (B6: `P`, `X`, `1`–`9`; mockups: `C`, `G`,
`T`, `Z`). Lightroom's `R` is Crop, which Framehive does not have (ADR 0042).

The marker belongs to the asset like the rating (LIBRARY-001, metadata): it is stored in
the library and written to the sidecar of every member as `framehive:KeepRaw`
(`docs/xmp-namespace.md`, ADR 0042). Marking writes `True`; unmarking removes the
property. A value `True` in any case counts as marked; a missing property, `False` and
any other text count as unmarked. A `framehive:KeepRaw` that is a struct or an array is
never overwritten: the change is refused for that asset and reported. Rebuilding the
index reads the marker back from the sidecars.

The filter bar (B14) has a Keep RAW group: *marked* and *not marked*.

**Remove RAWs.** A bulk command on the current view, that is the assets the grid shows
with its folder, filter and search. For every asset in the view that

- has a `Raw` member and a `Rendered` member,
- is not rejected, and
- is not marked Keep RAW,

it moves the `Raw` member and the RAW's own sidecar (`<raw>.xmp`, for example
`DSCF1234.RAF.xmp`) to the trash. The rendered member, its own sidecar, a shared
`<base>.xmp` (`DSCF1234.xmp`) and all metadata of the asset stay. Assets with only a
rendered member or only a `Raw` member are skipped, as are rejected and marked assets.
The asset remains, with its rendered member (LIBRARY-001, "Delete RAW member").

**Summary.** Before any file moves, a confirmation shows:

- the totals: assets, files and bytes freed (the sum of the sizes of the files that go
  to the trash);
- the group *RAW will be removed* with its count and a thumbnail per asset;
- the group *RAW stays (Keep RAW)* with its count and a thumbnail per asset of the view
  that has a `Raw` and a `Rendered` member, is not rejected and is marked;
- how many assets are skipped as rejected, as JPEG only and as RAW only;
- the list of files, collapsed.

Each group scrolls on its own. Clicking a thumbnail toggles the asset's Keep RAW marker,
as `R` would, and moves it to the other group; the totals and the confirm button follow.
Cancel moves no file; markers toggled in the summary stay toggled.

**Failures.** Assets are processed one by one, and a failure stops only its own asset.
The RAW goes first. If the RAW cannot be moved to the trash, its sidecar is not touched
and the asset stays complete. If the RAW moved but its sidecar cannot, the sidecar stays
where it is. Afterwards a report names every file that could not be moved, with the
reason, and the totals count only what was moved.

## Acceptance criteria

### LIBRARY-002.1 – R toggles the marker
- **Given** an unmarked asset `A` selected in the grid
- **When** the user presses `R`, and then `R` again
- **Then** `A` is marked after the first press and unmarked after the second

### LIBRARY-002.2 – The marker is independent of rating and pick state
- **Given** an asset rated 3 stars and rejected
- **When** the user marks it Keep RAW and then rates it 5 stars
- **Then** it is marked, rejected and rated 5 stars

### LIBRARY-002.3 – Marking writes `framehive:KeepRaw`
- **Given** an XMP packet with `xmp:Rating` 4
- **When** the asset is marked Keep RAW
- **Then** the packet has `framehive:KeepRaw` `True`, and `xmp:Rating` is still 4

### LIBRARY-002.4 – Unmarking removes the property
- **Given** an XMP packet with `xmp:Rating` 4 and `framehive:KeepRaw` `True`
- **When** the asset is unmarked
- **Then** the packet has `xmp:Rating` 4 and no `framehive:KeepRaw`

### LIBRARY-002.5 – Reading the marker
- **Given** packets whose `framehive:KeepRaw` is `True`, `true`, `TRUE`, `False`, empty,
  `yes`, or missing
- **When** the marker is read
- **Then** the first three are marked and the others are not

### LIBRARY-002.6 – A value of another form is kept
- **Given** a packet whose `framehive:KeepRaw` is an array
- **When** the asset is marked or unmarked
- **Then** the change is refused, and the packet is unchanged and counts as unmarked

### LIBRARY-002.7 – Every member's sidecar holds the marker
- **Given** an asset `DSCF1234.RAF` + `DSCF1234.JPG` with sidecar naming
  `original_with_extension`
- **When** the user marks it Keep RAW, and the index is then deleted and rebuilt
- **Then** both sidecars contain `framehive:KeepRaw` `True`, and the asset is marked

### LIBRARY-002.8 – Filter by Keep RAW
- **Given** assets `A` (marked) and `B` (not marked)
- **When** the filter Keep RAW *marked* is on
- **Then** the grid shows `A` only, and with *not marked* it shows `B` only

### LIBRARY-002.9 – Remove RAWs moves the RAW and its own sidecar
- **Given** an unmarked, not rejected asset with `DSCF1234.RAF`, `DSCF1234.RAF.xmp`,
  `DSCF1234.JPG` and `DSCF1234.JPG.xmp`, rated 4 stars
- **When** the user runs Remove RAWs on the view and confirms
- **Then** `DSCF1234.RAF` and `DSCF1234.RAF.xmp` are in the trash, and the asset remains
  with `DSCF1234.JPG`, its sidecar and the rating 4

### LIBRARY-002.10 – A shared sidecar stays
- **Given** an unmarked asset with `DSCF1234.RAF`, `DSCF1234.JPG` and the shared
  sidecar `DSCF1234.xmp`
- **When** the user runs Remove RAWs and confirms
- **Then** only `DSCF1234.RAF` is in the trash, and `DSCF1234.xmp` is unchanged

### LIBRARY-002.11 – Marked, rejected and single-member assets are skipped
- **Given** assets `A` (RAF+JPG, marked), `B` (RAF+JPG, rejected), `C` (JPG only),
  `D` (RAF only) and `E` (RAF+JPG)
- **When** the user runs Remove RAWs and confirms
- **Then** only the RAW of `E` is in the trash, and every file of `A` to `D` stays

### LIBRARY-002.12 – Only the current view
- **Given** two unmarked RAF+JPG assets, and a filter that shows only one of them
- **When** the user runs Remove RAWs and confirms
- **Then** only the RAW of the shown asset is in the trash

### LIBRARY-002.13 – The summary shows totals and two groups
- **Given** a view with `A` (RAF 30 MB + JPG, sidecar `A.RAF.xmp` 4 KB), `B` (RAF+JPG,
  marked) and `C` (JPG only)
- **When** the user runs Remove RAWs
- **Then** the summary shows 1 asset, 2 files and the bytes of `A.RAF` and `A.RAF.xmp`;
  *RAW will be removed* (1) with a thumbnail of `A`; *RAW stays (Keep RAW)* (1) with a
  thumbnail of `B`; and 1 asset skipped as JPEG only

### LIBRARY-002.14 – A click in the summary toggles Keep RAW
- **Given** the summary of .13
- **When** the user clicks the thumbnail of `A`
- **Then** `A` is marked and shown under *RAW stays (Keep RAW)* (2), the totals show 0
  assets and 0 files, and clicking it again moves it back

### LIBRARY-002.15 – Cancel moves no file
- **Given** the summary of .13, in which the user clicked `B`
- **When** the user cancels
- **Then** no file has moved, and `B` is no longer marked

### LIBRARY-002.16 – A RAW that cannot be moved keeps its sidecar
- **Given** assets `A` and `B`, both unmarked RAF+JPG with own sidecars, and `A.RAF`
  cannot be moved to the trash
- **When** the user runs Remove RAWs and confirms
- **Then** `A.RAF` and `A.RAF.xmp` stay, `B.RAF` and `B.RAF.xmp` are in the trash, and
  the report names `A.RAF` with the reason

### LIBRARY-002.17 – A sidecar that cannot be moved is reported
- **Given** an unmarked RAF+JPG asset `A` whose `A.RAF.xmp` cannot be moved to the trash
- **When** the user runs Remove RAWs and confirms
- **Then** `A.RAF` is in the trash, `A.RAF.xmp` stays, the report names it with the
  reason, and the totals count one file

## Out of scope

- Undo of Remove RAWs: the trash is the way back.
- Removing the rendered member of many assets at once.
- The trash itself (freedesktop trash, macOS Trash): the library's file operations.

## Open questions

–

## History

| Date | Change |
|---|---|
| 2026-11-09 | Initial version, owner request; same rule as honeycomb (ADR 0042) |
