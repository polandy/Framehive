# 0014 – License: GPL-3.0-or-later

- Status: accepted
- Date: 2026-10-17

## Context

Framehive needs a license before the repository is created. The license must be
compatible with the GUI toolkit and the native libraries:

- Slint: GPLv3, royalty-free license or commercial.
- LibRaw: LGPL-2.1 or CDDL-1.0.
- libgphoto2: LGPL-2.1.

Comparable programs (Rapid Photo Downloader, digiKam, darktable) are GPL.

## Decision

Framehive is licensed under **GPL-3.0-or-later**.

- Slint is used under its GPLv3 option. The app needs no "Made with Slint"
  attribution.
- LGPL libraries are linked dynamically.
- Every source file carries SPDX copyright and license headers. Files that cannot
  carry a header (documents, specs, lock files, mockups) are covered by `REUSE.toml`.
  The license texts live in `LICENSES/` (REUSE specification).
- `cargo deny` allows only licenses compatible with GPL-3.0-or-later.

## Alternatives

- **MIT OR Apache-2.0:** most permissive. With Slint's royalty-free license the
  distributed app is still not purely permissive, and it needs an attribution.
- **MPL-2.0:** a middle ground, with the same Slint situation as above.

## Consequences

- Improvements to Framehive stay open source.
- Dependencies under GPL-incompatible licenses are excluded, and `cargo deny` checks
  this.
- Code under Apache-2.0 is fine (it is compatible with GPLv3). Code under
  GPL-2.0-only is not.
