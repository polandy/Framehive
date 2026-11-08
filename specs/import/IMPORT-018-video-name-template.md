# IMPORT-018 – Video name template

- Status: implemented
- Module: import
- Origin: R7
- Depends on: IMPORT-001, IMPORT-002, IMPORT-006, IMPORT-011, IMPORT-013

## Purpose

Videos are often filed differently from photos, for example by job instead of by day.
An import run can therefore name its videos by a template of their own.

## Behaviour

**Which template.** An import run has an optional video name template. A file whose
role is `Video` (IMPORT-013, "Which files are videos") is named by the video name
template; every other file, also a sidecar, by the name template. Without a video name
template, videos are named by the name template, as before.

**Rules of a template.** The video name template is a name template: IMPORT-001 says
what it may contain, how it renders and which templates it rejects. Its target path is
read below the file's own destination (IMPORT-013).

**Shared by both templates.** The conflict policy (IMPORT-003), the job code
(IMPORT-006) and the sequence numbers (IMPORT-002) are the same for photos and videos:

- A run whose name template or video name template uses `{job}` needs a job code; without
  one, it stops as IMPORT-006 says.
- One numbering covers all assets of the run, photos and videos, in shooting order
  (IMPORT-002, "Order"). A template without a sequence token shows no number, and its
  assets still take theirs, so the numbers of the other template keep gaps for them.
- The counter is the kind of the sequence token. When both templates have one, both must
  be of the same kind; otherwise the run does not start, nothing is written or reserved,
  and the error names both tokens, e.g. `the video template uses {seq_stored} but the
  name template uses {seq_today}; use one sequence kind in both`. The formats may
  differ. Every template with a sequence token must fit the last number of the run,
  else the run stops as IMPORT-002.7 says, naming that template.

**Photo and video targets never meet.** The last segment of a target ends with the
file's own extension (IMPORT-001), and no extension gives both the role of a photo and of
a video, so a photo and a video never get the same target file. Name conflicts are
resolved at each file's destination, as IMPORT-013 says.

**Where it is given.** The import profile key `naming.video_template` (IMPORT-011) and the
command-line option `--video-template` (CLI-012). The guided import (CLI-002) has no
video name template.

## Acceptance criteria

### IMPORT-018.1 – Videos are named by the video name template
- **Given** a source with `DSCF0001.RAF` and `clip.MOV`, the name template
  `{name}.{ext}` and the video name template `Clips/{name}.{ext}`
- **When** they are imported
- **Then** the targets are `DSCF0001.RAF` and `Clips/clip.MOV`

### IMPORT-018.2 – Without a video name template videos use the name template
- **Given** the source and name template of IMPORT-018.1 and no video name template
- **When** they are imported
- **Then** the targets are `DSCF0001.RAF` and `clip.MOV`

### IMPORT-018.3 – The video name template below the video destination
- **Given** the source of IMPORT-018.1, a video destination and the video name template
  `{date:%Y}/{name}.{ext}`, with `clip.MOV` captured in 2026
- **When** they are imported
- **Then** the video destination has `2026/clip.MOV`, and the import destination has
  `DSCF0001.RAF` only

### IMPORT-018.4 – One numbering for both templates
- **Given** the source of IMPORT-018.1 with `DSCF0001.RAF` captured before `clip.MOV`,
  the name template `{seq_stored:3}-{name}.{ext}`, the video name template
  `V{seq_stored:4}.{ext}` and the stored counter at 100
- **When** they are imported
- **Then** the targets are `101-DSCF0001.RAF` and `V0102.MOV`, and the stored counter is
  at 102

### IMPORT-018.5 – Different sequence kinds stop the run
- **Given** the name template `{seq_today:3}.{ext}` and the video name template
  `{seq_stored:3}.{ext}`
- **When** the import run starts
- **Then** it does not start, nothing is written, both counters are unchanged, and the
  error names `{seq_stored}` and `{seq_today}`

### IMPORT-018.6 – `{job}` in the video name template needs a job code
- **Given** the name template `{name}.{ext}`, the video name template
  `{job}/{name}.{ext}` and no job code
- **When** the import run starts
- **Then** it stops with an error that names `{job}`, and no file is copied

### IMPORT-018.7 – A video name template too narrow for the numbers
- **Given** the name template `{seq_stored:4}.{ext}`, the video name template
  `{seq_stored:2}.{ext}`, the stored counter at 98 and five assets
- **When** the import run starts
- **Then** it does not start, the counter stays at 98, and the error names the video name
  template and suggests `{seq_stored:3}.{ext}`

### IMPORT-018.8 – The profile key
- **Given** a profile with `naming.video_template: "{date:%Y}/{job}/{name}.{ext}"`, a
  profile without it, and a profile whose `naming.video_template` is
  `{date:%Y}/{unknown}.{ext}`
- **When** they are read
- **Then** the first has that video name template, the second has none, and the third is
  an error at the value with the message of IMPORT-001

## Out of scope

- A video name template in the guided import (CLI-002): it does not use import profiles
  yet.
- Picking the template by the asset rather than by the file, once grouping (LIBRARY-001)
  puts a video and its sidecar into one asset.
- Video metadata tokens such as codec or length (R3): a later spec.

## Open questions

–

## History

| Date | Change |
|---|---|
| 2026-11-08 | Initial draft; owner decision #72 (A): `naming.video_template` and `--video-template`, left out the name template applies, shared conflict policy, job code and sequence counters; one sequence kind in both templates follows from sharing the counters (D8) |
| 2026-11-08 | Implemented |
