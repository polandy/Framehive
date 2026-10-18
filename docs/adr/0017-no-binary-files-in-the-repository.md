# 0017 – No binary files in the repository

- Status: accepted; where decision 9 links review screenshots is superseded by ADR 0019
- Date: 2026-10-18

## Context

The test strategy needs large sample files: RAF, CR2 and DNG reference sets, HEIF and
video samples, and memory-card images (requirements 9.3, CORE-001, CORE-003,
CORE-004). Together they will be several gigabytes. Git keeps every version of every
file forever, so binary files make clones slow, cannot be reviewed as diffs, and
cannot be removed later without rewriting history.

The project owner wants the source repository to contain no binaries or blobs. The
rule must be enforced by `cargo xtask ci` (CLAUDE.md), not by review.

## Decision

1. **The repository contains text files only.** Every file is valid UTF-8 without NUL
   bytes and at most 256 KiB. `cargo xtask check-files` checks every file that Git
   tracks or would track, and `cargo xtask ci` runs it.
2. **Large test data lives in a separate repository**, `framehive-testdata`. Its files
   are published as immutable release assets (one archive per reference set, e.g.
   `fujifilm-v1.tar.zst`). A published asset is never replaced; a change is a new
   version.
3. **`testdata.lock` pins every file** with its URL, size and BLAKE3 hash. Public
   samples from raw.pixls.us are pinned directly at their source URL.
   `cargo xtask testdata fetch` downloads the pinned files, verifies the hashes and
   unpacks them into the test data cache. The cache is `.testdata/` in the repository
   root (ignored by Git), or the directory in `FRAMEHIVE_TESTDATA_DIR`. Tests get the
   path through that variable and never touch the real home directory (ADR 0006).
4. **Small fixtures stay in the repository as text**: XMP, YAML (`.expected.yaml`,
   cleaning scripts), JSON. Small binary inputs, such as a minimal TIFF or JPEG
   structure, are built by code in the test, not stored as files.
5. **Mockups stay in the repository** as HTML and JSON under `mockups/`, because they
   are text and change together with specs. They are viewed through the published
   design canvas. Screenshots and image exports are never committed.
6. **Generated binaries** (app icons in PNG/ICNS, bundles, screenshot test output) are
   produced by the build from text sources such as SVG, or stored as CI artifacts.
   The logo and icon sources are SVG files in the repository.
7. **Screenshot tests store hashes, not images.** The UI is rendered with the Slint
   software renderer, embedded fonts and a fixed window size and scale
   (architecture §9), so a screen renders to the same pixels on every machine. The
   expected result of each screenshot test is the BLAKE3 hash of its decoded pixels,
   pinned in `apps/gui/tests/screenshots.lock` with the screen name, theme and scale.
   - A failing test writes the actual image, and the reference image rendered from the
     base commit, plus a difference image, to the CI artifacts. A reviewer compares
     them there; the base rendering is possible because rendering is deterministic.
   - An intended change updates the hashes with `cargo xtask screenshots update`. The
     changed lines in `screenshots.lock` show in the diff which screens changed.
8. **Screenshots in the documentation are generated** by the same renderer when the
   documentation is built, never committed.
9. **Screenshots for reviews live in a separate repository**, `framehive-assets`. A
   pull request that changes the UI shows the change as 2–3 small, cropped screenshots,
   stored under `pr-<n>/` there and linked from the description. These images are
   disposable: unlike `framehive-testdata`, nothing pins them, so the repository may be
   pruned or recreated at any time.

## Alternatives

- **Git LFS:** keeps blobs out of Git objects, but ties the workflow to an LFS server,
  needs LFS on every machine and in CI, and is subject to storage and bandwidth quotas.
- **The project owner's Nextcloud:** full control, but a hosted CI can only reach it if
  the instance is public, and share links can break.
- **Object storage (S3, Backblaze B2):** robust, but needs an account and costs money.
  It remains an option if release assets ever stop being sufficient; only the URLs in
  `testdata.lock` would change.

## Consequences

- Tests that need reference files require `cargo xtask testdata fetch` first. CI caches
  `.testdata/` keyed by the hash of `testdata.lock`.
- The fetch command needs an HTTP client and BLAKE3. Its dependencies go through
  ADR 0009 when the command is implemented.
- Releasing the owner's own photos as test data happens in `framehive-testdata`,
  including the cleaning step (CORE-001).
- Any accidental binary, such as a screenshot or a core dump, fails CI before it can be
  committed through a reviewed change.
- A screenshot hash also changes when the pinned Slint version, the embedded fonts or
  the renderer settings change. Such an update changes `screenshots.lock` in the same
  commit, and the CI artifacts show the new renderings.
- Reviewing a screenshot change needs the CI artifacts, not just the diff. In return
  the repository does not grow with every UI change.
- `cargo xtask screenshots update` and the artifact upload are built together with the
  first screenshot test.
- Review screenshots need write access to `framehive-assets`. A broken link in an old
  pull request is accepted; the change itself is in the code and the screenshot hashes.
