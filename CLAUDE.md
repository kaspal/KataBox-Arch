# CLAUDE.md

Guidance for Claude (and any AI pair) working in this repo. Read it at the start of every session.

## What KataBox Arch is

A free, open-source, offline desktop app (Windows, then Linux) for practising system design, LeetCode style. You place real-product components on a free-form canvas, wire them, and run a deterministic traffic simulation that animates every request and grades the design against the problem's SLOs. It belongs to the KataBox family and mirrors KataBox's stack, tooling and rules. Decisions are summarized in `docs/adr/`.

## Layout

| Path | What it is |
| --- | --- |
| `apps/desktop/` | Tauri 2 app. `src/` = React 19 + TypeScript UI: `app/` shell, router, palette, dialogs; `pages/`; `solve/` the canvas editor (store, library, inspector, nodes, edges, `geometry/routeEdge.ts`, `wiring.ts`); `components/ui/` shadcn; `lib/api.ts` is the only door to the generated `bindings.ts`; `i18n/locales/`; `styles/tokens.css`. `src-tauri/` = thin Rust shell (commands, dialogs, window state, close guard; `icons/katabox-arch.svg` is the icon source) |
| `crates/ka-core` | Domain types (Design, Node, Edge, Group, Note), the component registry, the wiring table, design validation. No I/O |
| `crates/ka-store` | SQLite via sqlx (runtime queries only): `user.db` (WAL, migrations in `migrations/`) |
| `crates/ka-app` | App service behind the Tauri commands: `.kbarch` open/save, recent files, settings allow-list, window rules, typed `AppError` |

Later milestones add `ka-sim` (M1), `ka-judge` + `ka-pack` + `content.db` (M2), and `ka-review` (M4).

## Commands

```sh
cargo test                       # library crates (default-members)
cargo clippy --all-targets -- -D warnings
cargo clippy -p katabox-arch-desktop --all-targets -- -D warnings   # and again with --features updater
cargo fmt --all
cargo deny check licenses
KA_UPDATE_SNAPSHOTS=1 cargo test -p ka-core   # accept a reviewed change to the wiring matrix snapshot
pnpm install && pnpm build       # UI build (tsc + vite)
pnpm test                        # UI tests (Vitest + Testing Library; i18n parity, token contrast, routeEdge, wiring parity)
pnpm licenses:check              # npm license allow-list (mirrors deny.toml)
pnpm versions:check              # version equal in Cargo.toml / package.json / tauri.conf.json; MSI version rule
pnpm dev                         # run the desktop app; debug builds use <identifier>-dev data folders ("DEV DATA" badge)
pnpm dev:web                     # the UI alone in a browser (no backend: commands fail)
cargo run -p katabox-arch-desktop --features bindings --bin export-bindings  # regenerate bindings.ts and src/test/fixtures/registry.json
pnpm notices                     # third-party notices into src-tauri/resources (release builds refuse the placeholder)
```

## Rules

- Small PRs, one concern each, with tests. All checks green before merge (run them locally while GitHub Actions is blocked); the maintainer reviews every PR. Task branches only; never push to `main`.
- **Never change the CSP in `tauri.conf.json` or the capabilities (`core:default` only) without the maintainer's explicit sign-off in the PR description.**
- Business logic goes in `crates/`, never in `src-tauri` or React components. The canvas store (Zustand + zundo) is editing state only; anything persisted goes through commands, and Rust validates it.
- The UI never touches the file system; it calls typed Tauri commands through the generated `bindings.ts`, and `lib/api.ts` is the only module that imports them (a test checks it). Commands return `Result<T, AppError>` with a stable error code. File dialogs are opened from Rust (`tauri-plugin-dialog`); the UI may name a path only if it came from a dialog or Recents.
- The component registry and the wiring rules live once, in `ka-core`, as data. The UI gets them through the `registry` command and never duplicates them; `wiring.ts` is only the evaluator, kept in step with Rust by the parity test against `src/test/fixtures/registry.json`. App code never imports that fixture (a test checks it).
- Every component kind has its own glyph (`solve/glyphs.ts`, a test checks coverage) and, from M3, a Guide entry.
- The simulation is deterministic: no wall-clock time and no OS randomness in `ka-sim` or `ka-judge` (seeded PRNG from `ka-core` only).
- AI review never changes a score. The judge stays deterministic; the AI explains and challenges.
- No network at runtime except the AI review requests the user starts, sent to the provider they configured. No telemetry, ever.
- Every user-facing string goes through i18n (English and Spanish), including registry names (`registry.kind.<id>`, `registry.flavour.<id>`, `registry.prop.<key>`, `units.<unit>`): add keys to both `apps/desktop/src/i18n/locales/{en,es}.json` (keys are typed; tests check parity and that every registry id has a name). Show errors by their stable codes (`errors.<code>`). Colours always come with an icon or a label, and the tokens must keep the WCAG contrast test passing; text never sits on the desk surface.
- New data in `user.db` only via a numbered migration: a new `crates/ka-store/migrations/NNNN_*.sql` added to `MIGRATIONS`, with seeds in `migration_tests.rs`. Never edit a released migration.
- `.kbarch` changes follow ADR 0003: bump `SCHEMA_VERSION`, add a migration step in `ka-app/src/kbarch.rs` with a test file of the old version. Readers stay strict (`deny_unknown_fields`, size cap).
- Packaging: auto-update (cargo feature `updater`, `tauri.updater.conf.json`) and code signing stay off until the maintainer turns them on; never generate, commit or ask for a signing or updater key. Keep `@tauri-apps/api` and `@tauri-apps/cli` on the tauri crate's minor (2.12), and `rust-version` (1.99) at the repo's stable toolchain. New dependencies: exact pins at least 2 weeks old, on the licence allow-lists.
- Record significant decisions as a new ADR in `docs/adr/`.
