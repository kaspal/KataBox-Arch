# KataBox Arch

KataBox Arch is a desktop app for Windows and Linux for practising system design, LeetCode style. You place real-world components (load balancers, caches, queues, databases) on a free-form canvas, wire them, and run a deterministic traffic simulation that animates every request and grades the design against the problem's SLOs. It is local-first, with no account and no telemetry, and belongs to the KataBox family. Code is licensed MIT OR Apache-2.0.

> **Status:** pre-release (0.1.0, heading for 0.9.0 beta). Everything below works today except the AI review, which arrives in M4.

## What's inside

- **30 problems** in two sets, from URL Shortener to an order-matching exchange: each with a brief, targets, visible
  and hidden tests, three hints and an editorial. Submit a design and the judge scores it on correctness, cost and
  latency headroom; the same design always gets the same result.
- **A free-form canvas** with 25 kinds of component and their real-product flavours, Regions, AZs, autoscaling groups
  and sticky notes; wiring rules that explain themselves; undo, snap to grid, `.kbarch` files.
- **Live runs:** every request animated, cards showing their load, injected failures, a timeline you can scrub, live
  metrics and a breaking-point finder.
- **Learning:** the Component Guide (25 entries, concept cards, quizzes), 10 patterns, decision guides, a glossary, a
  numbers cheat sheet, guided samples, estimation drills, ten "Find the flaw" drills, five learning paths and spaced
  review of missed quiz questions.
- **Sharing:** problem packs (`.kbpack`, checked before install), Mermaid export, SVG and PNG images.
- **English and Spanish** throughout; high contrast and reduced motion; works offline.

See [docs/user-guide.md](docs/user-guide.md) for a tour, [docs/pack-authoring.md](docs/pack-authoring.md) to share
problems, and [docs/known-issues.md](docs/known-issues.md).
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
