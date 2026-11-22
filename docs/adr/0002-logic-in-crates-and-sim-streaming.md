# ADR 0002: Logic in crates, a thin shell, and how the simulation will stream

- Status: accepted (M0, 2026-11-21); the streaming part is the plan M1 implements

## Context

KataBox keeps product logic in Rust crates so it is unit-tested with `cargo test` and the UI only draws. Arch adds a simulation and a judge that must be deterministic and fast, and a canvas that must stay responsive while they run.

## Decision

- **Crates own the rules.** `ka-core` (no I/O) holds the domain types, the component registry, the wiring table and design validation. `ka-store` owns `user.db`. `ka-app` is the service behind every command: file access, path rules, the settings allow-list, typed errors. `src-tauri` only forwards commands, opens dialogs, sets the window title and guards closing.
- **Rules as data.** The registry and the wiring table are data, sent to the UI by the `registry` command. The UI evaluates the same table for instant feedback (ADR 0004) and Rust re-validates on save.
- **Paths.** The webview has `core:default` only. It may name a path only if it came from a Rust dialog this session or from the recent-files list.
- **Settings.** The UI may read and write only the allow-listed keys (`ka-app/src/settings.rs`). Rust-only keys such as `window.bounds` are refused.
- **Simulation (M1).** `ka-sim` is a seeded discrete-event engine with no wall-clock time and no OS randomness. A run takes an immutable snapshot of the design; editing during a run forks a new run. Live runs stream compact binary frames over `tauri::ipc::Channel` at no more than 30 Hz, and the canvas interpolates at 60 fps on one Canvas 2D layer that samples the same curves as the edges (`routeEdge`).
- **Close guard.** The UI reports unsaved changes with `set_dirty`. Rust holds a dirty close back (title-bar X, Alt+F4, taskbar Close) and emits `close-requested`. The UI asks Save / Discard / Cancel, then calls `confirm_close`. This needs no extra capability.

## Consequences

Everything that decides an outcome is testable without a webview. The UI stays a renderer and an editor; it can be wrong for a frame (instant feedback) but never wrong on disk.
