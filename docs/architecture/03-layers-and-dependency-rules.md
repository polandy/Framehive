# 3. Layers and dependency rules

Part of the [architecture](README.md).

Each crate declares its layer in `Cargo.toml`:

```toml
[package.metadata.framehive]
layer = "module"        # domain | kernel | service | module | module-ui | adapter | app | tooling
port = "metadata"       # adapters only: the service crate whose port it implements; none for ports of domain
ffi = true              # FFI adapters only: allows `unsafe` (see below)
```

Every crate has a `README.md` with a `## Purpose` section. Crates in the domain,
kernel, service, module, module-ui and adapter layers also have a `## Public API`
section. `cargo xtask check-architecture` checks both.

Allowed dependencies (row may depend on column):

| from ↓ / on → | domain | kernel | service | module | module-ui | adapter | app |
|---|---|---|---|---|---|---|---|
| **domain** | – | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ |
| **kernel** | ✓ | – | ✗ | ✗ | ✗ | ✗ | ✗ |
| **service** | ✓ | ✓ | ✓ (acyclic, only via public API) | ✗ | ✗ | ✗ | ✗ |
| **module** | ✓ | ✓ | ✓ | **✗** | ✗ | ✗ | ✗ |
| **module-ui** | ✓ | ✓ | ✗ | only its own module | ✗ (except `ui-kit`) | ✗ | ✗ |
| **adapter** | ✓ | ✓ | only the port it implements | ✗ | ✗ | ✗ | ✗ |
| **app** | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | – |

Additional rules:

- **Only the apps wire things together.** Only `framehive-gui` and `framehive-cli`
  know concrete adapters. Everywhere else, code is written against traits.
- **Domain has no I/O**: no dependency on `std::fs`, `std::net`,
  `std::time::SystemTime::now`, threads or an async runtime. Only small exceptions are
  allowed (e.g. `serde` derive), and only via ADR (ADR 0009).
- **No cycles**, not even between services.
- **No `unsafe`** except in adapter crates with FFI (`adapter-gphoto2`, …). There,
  every `unsafe` block is justified with `// SAFETY:` and covered by tests.
- **Keep the public API small**: the default is `pub(crate)`. What a crate exports is
  listed in its README under "Public API".
