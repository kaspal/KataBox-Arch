# Development

How to build, run and check KataBox Arch from source. The project rules (for people and AI pairs alike) are in
[CLAUDE.md](../CLAUDE.md), and the decisions behind them in [docs/adr/](adr/).

## Prerequisites

- **Rust**, stable (`rust-toolchain.toml`), 1.99 or later (the workspace's `rust-version`).
- **Node 22 or later** and **pnpm 10** (`packageManager` in `package.json`).
- **WebView2** on Windows (it comes with Windows 10 and 11), or WebKitGTK 4.1 on Linux
  ([linux-webkitgtk.md](linux-webkitgtk.md)).

## Workspace layout

| Path | What it is |
| --- | --- |
| `apps/desktop/` | The Tauri 2 app. `src/` is the React 19 + TypeScript UI (`solve/` is the canvas editor; `lib/api.ts` is the only module that calls the generated `bindings.ts`). `src-tauri/` is a thin Rust shell: commands, dialogs, window state, the close guard |
| `crates/ka-core` | Domain types, the component registry, the wiring rules, design validation. No I/O |
| `crates/ka-sim` | The deterministic traffic simulator |
| `crates/ka-judge` | The judge: tests, verdicts and scores |
| `crates/ka-pack` | Content tooling: validates `content/`, builds `content.db`, reads and writes `.kbpack` packs |
| `crates/ka-store` | SQLite (`user.db`) and its numbered migrations |
| `crates/ka-app` | The app service behind the Tauri commands: `.kbarch` files, recent files, settings, typed errors |
| `crates/ka-web` | The simulator compiled to WebAssembly, for a planned web viewer. Not in the default build: `cargo build --target wasm32-unknown-unknown -p ka-web` |
| `content/` | Problems, outage replays, the Guide, patterns and the rest of the built-in content (CC BY-SA 4.0) |
| `docs/` | User and author documentation, ADRs |

## Run

```sh
pnpm install
pnpm dev        # run the desktop app
pnpm dev:web    # the UI alone in a browser (no backend, so commands fail)
```

Debug builds keep their data in folders ending in `-dev` (for example `%APPDATA%\dev.katabox.arch-dev`) and show a
**DEV DATA** badge, so they never touch a real profile.

## Checks

Run these before opening a pull request:

```sh
cargo fmt --all
cargo clippy --all-targets -- -D warnings
cargo clippy -p katabox-arch-desktop --all-targets -- -D warnings   # and again with --features updater
cargo test                       # library crates (the default members; the desktop shell is built with -p katabox-arch-desktop)
pnpm build                       # UI build (tsc + vite)
pnpm test                        # UI tests (Vitest + Testing Library)
cargo deny check licenses
pnpm licenses:check              # npm licence allow-list
pnpm versions:check              # the version matches across Cargo.toml, package.json and tauri.conf.json
```

Other commands:

```sh
cargo run -p katabox-arch-desktop --features bindings --bin export-bindings   # regenerate bindings.ts and the registry fixture
KA_UPDATE_SNAPSHOTS=1 cargo test -p ka-core   # accept a reviewed change to the wiring matrix snapshot
cargo run -p ka-pack -- validate --run        # validate content/ and judge every reference and naive design
pnpm notices                                  # third-party notices for a release build
pnpm release:build                            # notices, then the installers
```

Adding content: [problem-authoring.md](problem-authoring.md) and [pack-authoring.md](pack-authoring.md). Packaging:
[packaging.md](packaging.md).
