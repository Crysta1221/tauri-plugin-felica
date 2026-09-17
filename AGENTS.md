# Agent Instructions

This repository is a Tauri v2 plugin (`tauri-plugin-felica`) with a JavaScript/TypeScript webview API for desktop PaSoRi readers.

## Quality gates

Changes must pass with **no warnings and no errors**:

- `bun run lint` (oxlint on `src/typescript`)
- `bun run build`
- `cargo check`

## Comments

Write all code comments in English.

## Design principles

- Separate concerns and keep the blast radius of a change small.
- Keep high cohesion so internals can change without spreading edits.
- Keep coupling loose and minimize dependencies.
- Abstract at stable boundaries so the code tolerates change.
- Remove redundancy; do not duplicate logic.
- Do not over-nest directories. Keep the layout easy to follow.

## Tooling

Use **bun** and **cargo** in this plugin package.

- JavaScript API build: tsdown (`bun run build`) from `src/typescript` into `dist-js/`
- Lint: oxlint (`bun run lint`)
- Labels: `bun run generate:labels` from `assets/*.csv`
- Rust: `cargo check` (and the crate build script)

Do **not** introduce Vite+ in this plugin package. Vite+ belongs only in `examples/`. See [examples/AGENTS.md](examples/AGENTS.md).

## Architecture

```
src/rust/lib.rs       Crate root (module declarations + plugin init)
src/rust/error.rs     Shared error type
src/rust/desktop.rs   Desktop FeliCa backend (session, exclusive access)
src/rust/hardware/    Sony PaSoRi USB backends
src/rust/protocol/    FeliCa frames
src/rust/cards/scan.rs    High-level scan (steps 0–9)
src/rust/cards/rf.rs      Polling / Request Service / Read / Search / Select
src/rust/cards/profile.rs CardProfile trait + registry
src/rust/cards/*.rs       Product parsers (transit, waon, edy, nanaco, quicpay, lite) + decode
src/rust/commands/    Invoke commands (`mod.rs` + `<name>.rs` per command)
src/rust/models/      Request/response and shared types
src/typescript/       Webview JS/TS API (`scan` → ScanResult, `poll` → FelicaCard handle)
src/typescript/labels/ Optional station/system maps (`tauri-plugin-felica-api/labels`)
src/rust/build.rs     Plugin command list for permission generation
dist-js/              Built ESM + CJS + declarations
permissions/          Tauri command permissions
examples/             Host app used to exercise the plugin (separate package)
```

- The Cargo crate is `tauri-plugin-felica` with `[lib] path = "src/rust/lib.rs"`.
- The npm package is `tauri-plugin-felica-api` at the repo root (`exports` point at `dist-js/`). Install this package with **bun**. The example app is a separate pnpm project; see [examples/AGENTS.md](examples/AGENTS.md).
- Add a command as `src/rust/commands/<name>.rs` and DTOs as `src/rust/models/<name>.rs`. Declare `pub(crate) mod <name>;` in `commands/mod.rs`, register `commands::<name>::<name>` in `lib.rs` (`generate_handler!`), and add the name to `src/rust/build.rs`.
- Shared types (card IDs, services, blocks) belong in `models/`, not in command files.
- `desktop.rs` stays at the crate root as the desktop backend.
- Keep the TypeScript API a thin, typed wrapper around plugin commands, plus local `ScanResult` / labels enrich. Do not put UI or app routing here.
