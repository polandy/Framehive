# 4. Module system

Part of the [architecture](README.md).

## 4.1 What a module is

A module is a business unit with its own responsibility, its own UI and its own
data. It consists of two crates:

```
modules/import/
├── import/          # business logic, use cases, ViewModels (Rust, no Slint)  layer = module
│   ├── README.md    # responsibility, events, commands, data, extension points
│   ├── src/
│   └── tests/
└── import-ui/       # Slint views, bound to the ViewModels                    layer = module-ui
```

ViewModels are plain Rust structs with state and actions. This makes all interaction
logic testable without a UI; `*-ui` contains only presentation and binding.

## 4.2 The module contract

```rust
/// Contract every Framehive module implements. Registered statically by the apps.
pub trait Module: Send + Sync {
    /// Stable, unique id, e.g. "import". Used for settings, storage namespaces and logs.
    fn id(&self) -> ModuleId;

    /// Called once at startup. The module receives only the capabilities it declared.
    fn start(&self, ctx: ModuleContext) -> Result<(), ModuleError>;

    /// Index schema owned by this module (own table prefix, own migrations).
    fn storage(&self) -> Option<StorageSchema> { None }

    /// Job kinds this module can execute in the background.
    fn job_kinds(&self) -> Vec<JobKindDescriptor> { Vec::new() }

    /// Settings section (typed, with defaults and validation).
    fn settings(&self) -> Option<SettingsSchema> { None }

    /// Headless commands contributed to `framehive <subcommand>`.
    fn cli(&self) -> Option<CliContribution> { None }
}
```

`ModuleContext` grants access to exactly the capabilities the module needs (event bus,
catalog queries, writing metadata, enqueueing jobs, settings) – nothing beyond that.
The GUI side (`*-ui`) similarly registers views, menu entries and keyboard shortcuts
via `ui-kit`.

## 4.3 Communication between modules

- **Events** (facts, past tense): `AssetsImported`, `MetadataChanged`,
  `VolumeWentOffline`, `JobFinished`, … Typed, defined in the kernel, versioned.
  Modules subscribe to what interests them. A module cannot know who is listening.
- **Commands** (intents): `OpenInLibrary { assets }`, `EnqueueImport { plan }`. A
  command has exactly one recipient; it is registered via the kernel, not through a
  direct call.
- **Shared data** only via services: all modules read assets through `catalog`,
  change metadata through `metadata`. No module reads another module's tables.

## 4.4 Data per module

- Each module owns its tables in the index (prefix `<module_id>_`) and its
  migrations. Migrations are forward-only, tested and idempotent.
- Since the index can be rebuilt from files at any time, each module describes how
  it reconstructs its data (`rebuild`), or marks it as a cache.
- File formats that a module writes into the library (e.g. albums under
  `.framehive/`) are YAML, with a versioned, documented schema.

## 4.5 Adding a new module

1. `cargo xtask new-module <id>` generates both crates from a template: README,
   module implementation, example ViewModel, conformance test, workspace entry.
   The template lives in `xtask/templates/module/` and grows with the kernel: until the
   module contract and `ui-kit` exist, the module implementation is the module id and
   the conformance test checks only that id.
2. Describe responsibility and boundaries in the README; create an ADR for new
   events or commands in the kernel.
3. Register in the apps (one line per app).
4. The module conformance test (`kernel::testing::module_conformance`) must pass: ID
   unique, startup has no side effects outside the context, migrations idempotent,
   rebuild yields the same state, settings have valid defaults.

Modules are registered **statically** (at compile time). Dynamically loadable
plugins are not planned: Rust has no stable ABI, and static registration is easier to
test. Should third-party plugins become necessary, they will get their own ADR (e.g.
via WebAssembly).
