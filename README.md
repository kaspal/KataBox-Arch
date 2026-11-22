# KataBox Arch

KataBox Arch is a desktop app for Windows and Linux for practising system design, LeetCode style. You place real-world components (load balancers, caches, queues, databases) on a free-form canvas, wire them, and run a deterministic traffic simulation that animates every request and grades the design against the problem's SLOs. It is local-first, with no account and no telemetry, and belongs to the KataBox family. Code is licensed MIT OR Apache-2.0.

> **Status:** early development (M0: the canvas editor, open and save). Simulation, problems and the Component Guide arrive in later milestones.

## Develop

Needs Rust 1.99+, Node 22+ and pnpm 10, plus WebView2 (Windows) or WebKitGTK 4.1 (Linux).

```sh
pnpm install
pnpm dev        # run the app (debug builds keep their data in separate -dev folders)
cargo test      # crates
pnpm test       # UI
```

See `CLAUDE.md` for every check and the project rules, and `docs/adr/` for the decisions.

[Leer en español](README.es.md)
