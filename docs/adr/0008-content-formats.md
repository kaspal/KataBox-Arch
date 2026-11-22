# ADR 0008: Content formats and `ka-pack`

- Status: accepted (M2.1, 2026-11-22)

## Context

Problems now, and the Guide, patterns, samples, concept cards, decision guides and glossary in M3, are content. They
are written by people, reviewed in pull requests, shipped inside the app, and read offline. They need to be:
- easy to write and diff;
- checked strictly before they ship;
- fast to read at runtime.

## Decision

### Source: plain files under `content/`
- **TOML** for structured data, **Markdown** for prose, and **`.kbarch`** for designs.
- **Both languages.** Every human-readable field has an English and a Spanish version (`title` / `title_es`,
  `statement.md` / `statement.es.md`).
- **Problems:** `content/problems/<id>/` holds:
  - `problem.toml`: `ka_core::problem::ProblemDef`. It contains the NFRs, request types, externals, tests (with
    traffic, chaos, multipliers and per-test pass overrides) and the pass criteria;
  - the statement, the hints (`## 1`, `## 2` and `## 3`) and the editorial, in EN and ES;
  - `reference.kbarch` and `naive.kbarch`.
- **Chaos selectors** are written flat: `{ kill = { kind = "sql", role = "primary", at = 30 } }`.

### Validation (`ka-pack validate`)
- **Structural checks**, fast and needing no simulation:
  - files are present and parse;
  - the id matches the folder, and set and number are unique;
  - EN/ES parity: titles, test names, three hint tiers, and the same number of `##` sections in each statement and
    editorial;
  - request type shares add up to 1; routes name only declared externals;
  - at least one visible and one hidden test;
  - the reference and naive designs pass design validation, and every request type has a path through both;
  - `requires` kinds are in the reference;
  - no `TODO(ka-pack)` markers.
- **`--run`** (from M2.2) also judges: the reference is Accepted on every test, and the naive design fails at least
  one hidden test.

### Shipping: `content.db`
- **Schema 1**, in `ka-store::content`:
  - read-only, with `PRAGMA user_version = 1`; the app refuses a newer file;
  - tables `meta`, `problems`, `guide`, `patterns`, `samples`, `concepts`, `decide` and `glossary`;
  - structured columns are JSON strings, so `ka-store` doesn't depend on the content types.
- **Built at desktop build time.** The desktop build script (`src-tauri/build.rs`) runs `ka-pack build`
  (structural validation only, review A5) into `resources/content.db`, which `tauri.conf.json` bundles. The file is
  gitignored. The CSP and capabilities are untouched.
- **Metadata:** `meta` records the content hash (SHA-256 over every file, in path order), the schema and the app
  version.
- **`ref_cost` and `ref_p95_ms`:** each problem's row stores the reference design's monthly cost and p95 on the
  first test. The cost feeds the cost score (ADR 0007); both place the reference on the cost vs speed chart, which
  the brief sends only once the problem is solved, with the reference design.
- **Hidden tests stay in Rust.** A problem's full definition (`def`) is read by Rust only: the UI receives hidden
  tests by name, never their traffic or chaos.

### Tools
`ka-pack new <id>` scaffolds a folder from templates marked `TODO(ka-pack)`. `validate` and `build` work as above.
The arguments are parsed by hand, as in KataBox.

## Consequences
- **Reviewable.** Content changes are plain-text diffs, checked by the same rules locally and at build time.
- **Size.** `content.db` adds a few hundred KB to the bundle; the report records the size at each gate.
- **Schema changes.** Any new field is a `content.db` schema bump with a round-trip test, by the rules in
  `next-prompt.md`.
