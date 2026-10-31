# 0022 – `jiff` as the time library

- Status: proposed
- Date: 2026-10-31

## Context

The import needs two times that the standard library cannot give in local time:

- `{imported:<format>}` renders the time the import was queued in the machine's local
  time (IMPORT-001).
- A file without a capture time, such as a video in milestone 1, is named by its
  modification time, which the file system stores as an instant in UTC. Naming it needs
  the local UTC offset at that instant, including daylight saving time.

`std::time::SystemTime` gives the instant, but no time zone. The offset comes from the
system time zone database (`/etc/localtime`, `TZ`, `/usr/share/zoneinfo` on Linux and
macOS). ADR 0009 named `time` as the first candidate and `jiff` as the alternative to
evaluate. Architecture §6 still mentions `time::OffsetDateTime`.

## Decision

1. **`jiff` is the time library.** It is used only in `adapter-clock`, which
   implements the `Clock` port of `domain` (ADR 0020): the current instant as a
   `Timestamp`, and the local `UtcOffset` at any `Timestamp`.
2. **`domain` stays std-only.** `Timestamp` and `UtcOffset` are our own small types;
   `naming` turns them into a `LocalDateTime`. No other crate sees a `jiff` type, so a
   later switch changes one crate.
3. **Scope:** `default-features = false`, features `std`, `tz-system` (find the system
   time zone) and `tzdb-zoneinfo` (read the system time zone database).

| Criterion (ADR 0009 §3) | `jiff` |
|---|---|
| Origin | Andrew Gallant (BurntSushi), author of `regex`, `ripgrep` and `walkdir` |
| Adoption | Fast-growing, used by `uv` and other widely used tools |
| Maintenance | Regular releases |
| Stability | **Version 0.2, before 1.0.** Exception, see below |
| License | Unlicense OR MIT |
| Scope | Pure Rust, no native code; the chosen features add no dependency on Linux and macOS |

**The exception for stability.** ADR 0009 asks for version ≥ 1.0 or a de-facto standard
with a stable API. `jiff` meets neither yet. It is accepted because its author has a
long record of stable, well-maintained crates, its API is modelled on the TC39 Temporal
proposal and declared to head for 1.0, and it is confined to one adapter with two
functions. A breaking release costs a change in `adapter-clock` only.

## Alternatives

- **`time`:** version ≥ 1.0, but its local offset (`local-offset`) is refused in
  multithreaded programs on Unix, because reading the environment is unsound there. The
  import will run beside other threads.
- **`chrono`:** established, but a larger API with historical pitfalls, and its local
  time zone support pulls in further crates on macOS.
- **Our own reader for TZif files:** about 300 lines plus POSIX `TZ` rules for future
  transitions, the macOS paths and the `TZ` variable; error-prone for little gain.
- **UTC instead of local time:** simple, but folders would not match the photographer's
  calendar day, which is the point of `{date:%Y-%m-%d}`.

## Consequences

- `dependencies.yaml` gains `jiff`, allowed only in `adapter-clock`.
- Architecture §6 says that times are explicit `Timestamp` and `UtcOffset` values and
  that only `adapter-clock` reads the system time zone.
- The candidate `time` in ADR 0009 is not used; its table stays as written.
- When `jiff` reaches 1.0, the exception ends; the upgrade is a normal dependency update.
