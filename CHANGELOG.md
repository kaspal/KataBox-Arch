# Changelog

All notable changes to KataBox Arch are recorded in this file. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- **Autoscaling groups** are frames around compute cards, with a scaling policy (flavour,
  minimum and maximum instances, target utilisation, scale-out delay) edited in the Inspector.
- Edges that share a side of a card spread out along it instead of meeting at one point.
- SQL databases have a **Failover time** property.
- **Live runs in the Sandbox:**
  - **Run** plays the design against normal traffic or a ×4 spike, generated from the clients' request rates.
  - The **Metrics** tab shows live p95 latency, throughput, error rate and cost per month, plus the busiest
    components.
  - Pause, resume, stop and speed (0.25× to 8×).
  - Editing the design during a run restarts it with the change.
  - Requests travel the connections as coloured dots, one colour per request type. Cache hits flash and dropped
    requests show a cross.
  - Each card shows how busy it is and its queue, as text and a coloured bar; overloaded cards shake.
  - With reduced motion, connections show their requests per second instead of moving dots.
  - **A timeline under the canvas:** play/pause, the clock, a scrubber that jumps to any moment, the speed, and a
    marker for each failure.
  - **Inject** breaks the selected card at the current moment (kill it, slow it down, empty a cache) or triples the
    traffic. The run continues from there.
  - **A "Kill a node at 20 s" traffic preset.**
  - The Metrics tab plots p95 latency per second.
  - **Stale reads.** Reads served by a lagging replica, or by a cached copy older than the last write, are counted and
    shown as a Metrics tile.
  - **Find breaking point** (Metrics tab) pushes the traffic up until the design fails, then reports how much it
    holds and the first component to give way.
  - **The simulation models failures:**
    - dead cards are noticed by health checks after a few seconds;
    - databases fail over to a replica;
    - a dead cache turns into misses;
    - autoscaling groups add and remove instances as load changes.

- **Problems:**
  - **The catalogue:** search, difficulty and topic filters, status and best score, plus Continue and Random
    problem.
  - **Each problem opens the editor with a Brief tab:** the statement, targets, traffic, external providers, its
    tests (hidden ones by name only), hints you reveal one at a time, and the editorial once solved.
  - **Drafts** save as you work.
  - **Submit** judges the design on every test. Results show the verdict, a score (correctness, cost, latency), each
    test's numbers, the bottleneck of a failing test, and "Replay at failure", which plays that test from just
    before it broke.
  - **Home** shows the problem to continue, the recommended next one, and your streak.
  - **Opening a design file made for a problem** opens it on that problem's page.
  - **Once a problem is solved:**
    - **Compare with reference** shows your design and the reference side by side, with linked zoom and pan, or
      overlaid, the reference drawn faintly behind yours;
    - **a cost vs speed chart** places every accepted submission against the reference.
  - **A failed submission's results** offer the next hint.
- **The Component Guide:**
  - **25 entries**, one per component, in English and Spanish. Each covers what it does, when to use it and when
    to skip it, how the simulator models it (every number read from the simulator itself), the products compared,
    common pitfalls, trade-offs, practice problems, related components, an interview tip and vendor sources.
  - **The index** groups entries by category, with search, a "Start here" path and read marks.
  - **Open entries from anywhere:** the library's info button shows a quick look, the Inspector's "Learn more"
    opens the entry at "In the simulator", and Ctrl+K finds every entry.
- **Samples:**
  - **Five animated walkthroughs:** Hello, Traffic (the first-run tour), URL Shortener at scale, News Feed:
    fan-out, Chat with WebSockets, and Flash Sale. Every number they call out is checked against the simulator.
  - **Home** shows them under "Watch how it works". The card under the pointer, or the first one, plays a live
    6-second preview; the others are still pictures, and nothing moves with reduced motion.
  - **Home offers the tour once**; it always stays in Samples.
  - **The player** pans the camera to each step and dims what isn't in focus. Each step has a caption, plays live
    traffic, and calls out the number that changed.
  - Step with Previous and Next, the step dots or the arrow keys, and Replay step plays a step again.
  - **Try it yourself** copies the step's design into the Sandbox.
  - **Set 1:** 15 problems, from URL Shortener to Online Judge, each with a statement, hints and an editorial in
    English and Spanish.

### Changed
- Design files are format version 2. Version 1 files still open: an Autoscaling card becomes a
  frame around what it scaled, and two retired properties (SQL read replicas, CDN hit rate) are
  dropped.
- The Inspector's Label field shows the product name as its placeholder.
- "Fit the design" never zooms out below 80%, so card text stays readable.
- **Calibrated defaults:**
  - Component defaults are calibrated per product, and every figure is listed with its source in
    `docs/calibration.md`. Examples: PostgreSQL's 100 connections, MySQL's 151, Sidekiq's 5 threads, Cloud Run's
    limits.
  - The Inspector tags these figures as "ballpark" or "cited", in place of the old "placeholders" note.

### Fixed
- Saving a design failed after the format moved to version 2 (the editor still wrote version 1).
- A layout loop that re-measured the canvas every frame whenever zones or notes were on screen.
- React Flow's "parent container needs a width and a height" warning when opening the Sandbox.
