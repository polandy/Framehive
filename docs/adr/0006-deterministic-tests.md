# 0006 – Deterministic tests, no retries

- Status: proposed
- Date: 2026-10-17

## Context

Without human review, tests are the most important safety net. Flaky tests destroy
trust and tempt agents to ignore failures.

## Decision

- Tests are deterministic: `ManualClock`, fixed seeds, a deterministic job executor,
  dedicated temp directories, an explicit environment (TZ, locale, HOME), no network.
- `cargo nextest` with `retries = 0`. A flake is a top-priority bug.
- Nightly: the suite runs 20 times in random order.
- `std::thread::sleep`, `SystemTime::now`, `Instant::now` are forbidden in tests and
  domain logic via Clippy.

## Consequences

- Somewhat more effort when writing concurrent tests.
- A red CI always means a real bug.
