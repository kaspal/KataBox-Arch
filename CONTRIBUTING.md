# Contributing to KataBox Arch

Thanks for helping! The easiest ways to contribute, from small to large:

1. **Report a problem with a problem.** A reference that seems wrong, an unclear statement, a naive design that
   passes when it shouldn't, or a simulator figure you can cite better: open an issue with the problem id and what
   you saw.
2. **Translate.** UI strings live in `apps/desktop/src/i18n/locales/` (English and Spanish first). Problems have
   `statement.es.md`, `editorial.es.md` and `hints.es.md` next to the English files; Guide entries have `entry.es.md`.
3. **Add a problem.** Run `cargo run -p ka-pack -- new <id>` and fill in every TODO. The rules are in
   [docs/problem-authoring.md](docs/problem-authoring.md). `ka-pack validate --run --problems <id>` must pass (the
   reference is Accepted on every test and the naive design fails a hidden one), and
   `ka-pack seeds --problems <id>` shows how it holds across seeds. Problems must be original. Content is licensed
   CC BY-SA 4.0.
4. **Calibrate.** Every simulator figure has a row in [docs/calibration.md](docs/calibration.md); a better source for
   a `model` or ballpark row is very welcome.

## Development

See [docs/development.md](docs/development.md) for setup and commands, and [CLAUDE.md](CLAUDE.md) for the
architecture rules that apply to every contributor, human or AI.

- Keep pull requests small and focused, with tests.
- `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `pnpm test` and `pnpm build` must pass.
- A change to the simulator or the judge keeps every shipped verdict, unless it says why one moves.
- Changes to the CSP or the app's capabilities need the maintainer's explicit approval.

By contributing you agree that your code is licensed under MIT OR Apache-2.0 and your content under CC BY-SA 4.0.
