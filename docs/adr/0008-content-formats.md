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
- **Samples (M3.2):** `content/samples/<id>/` holds `sample.toml` (`ka_core::story::SampleDef`) and the `.kbarch`
  snapshots its steps name.
  - The sample has a title and summary in EN and ES, a gallery `number`, an optional `problem` (whose tests the
    steps may play) and `tour = true` on the first-run tour (at most one).
  - Each `[[step]]` has a snapshot, a title and caption in EN and ES, `focus` (card ids in the spotlight), an
    optional `camera = { x, y, zoom }` (a point centred in the view), an optional `run` (`test` or `preset`, with
    `start_s` and `speed`), and an optional `callout` (`metric`, `before`, `after`, `target`).
  - **Callout numbers are written in the content**, so building stays free of simulations (review A5).
    `validate --run` simulates each such step the way the player plays it (same scenario, seed and sampling): `after`
    on the step's design, `before` on the previous step's. Each must be within 15% of the simulator, or within a
    small absolute floor.
  - **Concept cards (M3.5)** use the same format in `content/concepts/<kind>/`: the id is a component kind, none
    is a tour, and callouts are checked the same way. `content.db` stores each as a `concepts` row (the sample row
    as JSON).
  - **A callout metric for queues:** `lost` counts async messages that failed every retry with no dead-letter
    queue.
- **Decision guides and glossary (M3.7):**
  - **Flows:** `content/decide/<id>.toml` (`ka_core::decide::DecideFlow`) has questions whose answers lead to
    another question or end at a component kind. The validator refuses loops, unreachable questions and answers
    with neither (or both) ends.
  - **The glossary:** `content/glossary.toml` holds `[[term]]` tables. Every English term must appear somewhere
    else in the content, so the list stays the app's real vocabulary.
- **Patterns (M3.6):** `content/patterns/<id>/` holds `pattern.toml` (`ka_core::pattern::PatternEntry`, every
  text with an `_es` twin), `pattern.md` / `pattern.es.md` ("How it works"), and `template.kbarch`.
  - `in_simulator` says plainly how much of the pattern the simulator runs.
  - The template must pass design validation, contain every kind the pattern names, and belong to no problem.
- **Component Guide (M3.4):** `content/guide/<kind>/` holds `entry.toml` (`ka_core::guide::GuideEntry`), plus
  `entry.md` and `entry.es.md` ("What it does").
  - **The fields:**
    - the one-liner;
    - when to use it and when to skip it;
    - the simulator rows and the formula;
    - the flavours compared, pitfalls (optionally `seen_in` a problem), gains and costs;
    - `works_with` kinds and `practice` problems;
    - an interview tip;
    - one source per flavour.

    Every text field has an `_es` twin.
  - **No numbers in entries.** Each `[[sim]]` row names one registry value: `prop` (a prop the simulator reads),
    `sim` (a hidden simulator default) or `cost` (a price). The page shows that value for every flavour, straight
    from the registry, so the Guide and the simulator can't drift. Notes may not contain digits.
  - **Props the simulator ignores** (`ka_pack::guide::NOT_SIMULATED`: eviction, persistence, replicas, retention
    and the like) may not appear as simulator rows. A test fails if `ka-sim` starts reading one, so the list stays
    true.
  - **Sources are vendor documentation roots**, written without fetching (unattended runs have no network), so
    each is `checked = false` until someone opens it. The page says so, and shows the link as text to copy, since
    the app opens nothing on the network.

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
- **Samples:** the id matches the folder; numbers are unique; at most one tour; text in both languages; every
  snapshot exists, is used, and passes design validation; focus ids are cards of the step's snapshot; a run names
  exactly one of a test of the sample's problem or a known preset, with a speed of 0.25–8 and a start inside the run;
  a callout needs a run, and the first step has no `before`.
- **`--run`** (from M2.2) also judges: the reference is Accepted on every test, and the naive design fails at least
  one hidden test. It also checks every sample callout against the simulator.
- **Guide:**
  - every registry kind has exactly one entry, in the folder named after it;
  - lists have the same length in EN and ES, and the prose the same number of paragraphs;
  - simulator rows name real, simulated registry values, with no digits in their notes;
  - compared flavours belong to the kind;
  - related kinds and problems exist;
  - every flavour has an `https://` source.

  `--verbose` lists the sources still unchecked.
- **`--kind samples`** (or `guide`) checks only that kind of content; with no `--kind` (and no `--problems`),
  everything is checked.

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
