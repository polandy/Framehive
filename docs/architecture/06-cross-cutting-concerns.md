# 6. Cross-cutting concerns

Part of the [architecture](README.md).

| Topic | Rule |
|---|---|
| Errors | Libraries: own error types with `thiserror`, every variant documented. `anyhow` only in apps and `xtask`. No panics in libraries (`unwrap`/`expect` forbidden by Clippy except in tests). |
| Logging & diagnostics | `tracing` with structured fields; spans per job and file. No `println!`. Diagnostic export for error reports (N24). |
| Concurrency | Standard-library threads and channels; an async runtime only when an adapter strictly needs one (ADR 0009). Business logic stays synchronous and pure. Cancellation via cancellation tokens, never by killing threads. |
| Time | Only through the `Clock` port. Times are explicit `Timestamp` and `UtcOffset` values (`domain`), never implicitly the system time zone; only `adapter-clock` reads it (ADR 0022). |
| IDs & randomness | Only through the `IdGenerator` and `Rng` ports (fixed seed in tests). |
| Paths | Never as `String`; `Utf8PathBuf` for our own paths, `PathBuf` at file-system boundaries. Inside the library, always volume ID + relative path (S2). |
| Configuration | YAML, typed, with schema version and migration; unknown keys are rejected. A published JSON Schema provides editor completion. Keys are descriptive, full words; units are explicit. See ADR 0011. Location follows XDG (Linux) or `~/Library/Application Support` (macOS). |
| Internationalization | All UI text via Slint translations (gettext); German and English from MVP onward. |
| Accessibility | Every interactive element has an accessible name; keyboard operation for everything (used in UI tests). |
| Security | No network access except map tiles and model downloads, both of which can be disabled. Hooks run without shell interpolation. |
