# ADR 0001: Stack and alignment with KataBox

- Status: accepted (M0, 2026-11-21)

## Context

KataBox Arch is a sibling of KataBox (the coding-practice app). One maintainer works on both with Claude, so shared tooling, versions, checks and release steps carry lessons from one to the other.

## Decision

Arch uses KataBox's stack at `main` f17a35d, the same versions and the same rules:

- **Shell:** Tauri 2.12, Rust edition 2024, MSRV 1.99, a Cargo workspace of small crates (`ka-*`) and a thin `src-tauri`.
- **UI:** React 19, TypeScript ~6.0, Vite 8, Tailwind 4.3 + shadcn/ui (new-york) with `@theme inline` hex tokens, TanStack Router (hash history) and React Query, cmdk, motion, react-resizable-panels v4, i18next (EN + ES), Vitest 5.
- **IPC:** tauri-specta typed commands; `lib/api.ts` is the only door to `bindings.ts`.
- **Checks:** `cargo deny`, `licenses:check`, `versions:check`, bindings drift, contrast and i18n parity tests. KataBox's scripts are copied and adapted.
- **Lockfiles:** `pnpm-lock.yaml` and `Cargo.lock` were seeded from KataBox's, so every shared package resolves to the version KataBox ships.

New for Arch (all MIT or OFL-1.1, exact pins at least two weeks old on 2026-11-21):

| Package | Pin | Why |
| --- | --- | --- |
| `@xyflow/react` | 12.12.1 | The canvas editor (13.x was under two weeks old). It brings its own `zustand` 4.5.7; that copy never shares a store with ours |
| `zustand` | 5.1.0 | Editor state |
| `zundo` | 2.3.0 | Undo/redo for that state |
| `@fontsource-variable/cormorant-garamond`, `geist`, `geist-mono` | 5.3.0 | Display, UI and number faces |
| `@fontsource/kalam` | 5.3.0 | Sticky notes (no variable build) |

Fonts are bundled, Latin and Latin Extended subsets only (`styles/fonts.css`), because the CSP allows no network fonts.

**Look.** The "paper desk" palette from the spec, with these AA adjustments: a new `amber-ink` (#946016) for amber text and glyphs; terracotta text uses `terracotta-ink`; yellow, amber and terracotta are fills only; dark `danger` is #eb7a72; dark `primary-foreground` is the desk colour. High-contrast `.hc` / `.dark.hc` themes reach 7:1 for text. Text never sits on the desk surface.

**Versions.** 0.x releases are plain `M.m.p` (no pre-release tags; minors ≤ 98) and map to the same MSI version. KataBox's rule then maps `1.0.0-beta.N` to `0.99.N`, above every 0.x build.

## Consequences

Upgrades are planned together with KataBox. Anything new must pass the same licence allow-lists, and new npm packages keep the exact-pin, two-week rule.
