<!--
SPDX-FileCopyrightText: 2026 Framehive contributors
SPDX-License-Identifier: GPL-3.0-or-later
-->

# kernel

Layer: `kernel` (architecture §3)

## Purpose

The kernel holds the module contract, the module registry, the event bus and the
commands (architecture §4). It is the only way modules interact. The scaffold contains
the module id; the contract itself follows with the first module.

## Public API

| Item | Meaning |
|---|---|
| `ModuleId`, `ModuleIdError` | Validated, stable module identifier (architecture §4.2) |

## Dependencies

- `thiserror` (ADR 0016)
