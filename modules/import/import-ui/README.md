<!--
SPDX-FileCopyrightText: 2026 Framehive contributors
SPDX-License-Identifier: GPL-3.0-or-later
-->

# import-ui

Layer: `module-ui` (architecture §3, §4)

## Purpose

Views of the import module, bound to the ViewModels of the `import` crate. It holds
only presentation and binding; all interaction logic stays in `import`
(architecture §4.1). The crate is empty until `ui-kit` and the Slint views exist.

## Public API

None yet.

## Dependencies

- `import`
