<!--
SPDX-FileCopyrightText: 2026 Framehive contributors
SPDX-License-Identifier: GPL-3.0-or-later
-->

# adapter-clock

Layer: `adapter`, for the `Clock` port of `domain` (architecture §3). It depends only
on `domain`, so its manifest declares no `port`.

## Purpose

Implements `domain::Clock` with the system clock and the system time zone (ADR 0020
§4, ADR 0022). The import uses it for the time an import was queued and to name files
without a capture time by their modification time in local time. It is the only crate
that reads the system time zone, and no other crate sees a `jiff` type, so a switch of
the time library changes this crate alone.

- `now()` is `jiff::Timestamp::now()`, converted to `domain::Timestamp`.
- `local_offset(at)` is the offset of the system time zone at `at`, including daylight
  saving time and earlier rules of the zone, converted to `domain::UtcOffset`.
- The time zone is read once, when the clock is created, from `TZ`, `/etc/localtime`
  and the system time zone database. If it cannot be found, local time is UTC.

## Public API

| Item | Meaning |
|---|---|
| `SystemClock` | `domain::Clock` with the system time zone; `SystemClock::new()` or `Default` |

## Tests

`domain` has no conformance suite for `Clock`, and a test against the wall clock or
the machine's time zone would not be deterministic (CLAUDE.md rule 4). The unit tests
build the clock with a fixed time zone from a POSIX `TZ` rule
(`CET-1CEST,M3.5.0,M10.5.0/3`, no time zone database needed) and check the offset in
winter, in summer, at both transitions and before 1970, and the conversions between
`jiff` and `domain`, including fractions of a second before the epoch.

## Dependencies

- `domain`
- `jiff` (ADR 0022): `default-features = false`, features `std`, `tz-system`,
  `tzdb-zoneinfo`
