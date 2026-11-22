# Changelog

All notable changes to KataBox Arch are recorded in this file. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- **Set 2: 15 more problems** (#16–30), from Image Thumbnails to an Order-Matching Exchange, each with hints, an
  editorial (with "What the simulator leaves out" where it matters) and estimation drills. Learning paths include
  them.
- **Problem packs:** import a `.kbpack` with a preview (author as stated, SHA-256, problems); every pack is checked
  (files, text and a judged run of every design) before it installs; a badge and a source filter on Problems;
  uninstall in Settings keeps your history. `ka-pack pack` builds packs.
- **Export:** copy or save a design as a Mermaid diagram; save it as an SVG or PNG image.
- **Related in KataBox** on Guide entries, patterns and problems.
- **Results and briefs show p99 limits** where a test has one.
- **Accessibility:** cards and connections have full names, the side panel's tabs follow the ARIA pattern, and run
  changes are announced. Reduced motion now follows the system everywhere.
- **Simulator:** unfinished work at the end of a run counts at its age; reads stay in the caller's Region;
  zone tests can target the Region holding the primary.
- **Docs:** user guide (English and Spanish), pack and problem authoring guides, known issues.
- **Practice modes:**
  - A **Numbers cheat sheet** (31 figures, with sources), linked from every problem brief.
  - **Estimation drills:** 37 back-of-the-envelope questions for Set 1, with worked solutions, an "Estimate first"
    panel in each brief and a drill page.
  - **Find the flaw:** 10 designs that pass a normal day but hide one flaw a test exposes.
  - **Learning paths:** 5 ordered tracks through the Guide, patterns, problems and drills; Home recommends the next
    step of the path you follow.
  - **Quizzes** at the end of every Guide entry and pattern (105 questions). Missed questions come back for spaced
    review (1, 3, 7 and 21 days).
- **Simulator:** worker leases (redelivery after a visibility or session timeout), change data capture, cost per
  shard, DynamoDB write units, SQL "Read from", provider rate cuts, link partitions between zones, and p99 targets.
- Glossary terms show their definition on hover or focus.
- 12 more concept cards.
- Properties left at their default follow calibration; the Inspector marks defaults and can reset a property.
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
  - **Concept cards** for the six "Start here" components (Load Balancer, App Server, Cache, SQL Database,
    Queue, CDN) play inline in their entries, with live traffic and checked numbers.
  - **Open entries from anywhere:** the library's info button shows a quick look, the Inspector's "Learn more"
    opens the entry at "In the simulator", and Ctrl+K finds every entry.
- **"Which one do I need?"** Four short decision guides (SQL or NoSQL, queue/stream/pub-sub, cache or CDN, load
  balancer or API gateway) that end at the right Guide entry, with the reason.
- **A glossary** of 42 terms the app uses, searchable, each linked to its Guide entry where there is one.
- **The pattern library:** 10 patterns (cache-aside, read replicas, fan-out, transactional outbox, saga, CQRS,
  sharding, circuit breaker, backpressure, rate limiting). Each page covers the problem it solves, how it works, when
  to use it, gains and costs, and how much of it the simulator shows. **Insert into the Sandbox** adds the pattern's
  cards to your design, centred in the view, as one undo step.
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
- **Prices are list prices** (us-east-1, on demand) for DNS, SQS, RabbitMQ, Kafka (MSK), Kinesis, SNS, Google
  Pub/Sub, Redis Pub/Sub, S3, GCS, DynamoDB, Cassandra and MongoDB, each cited in `docs/calibration.md`: reads and
  writes are priced apart, shards and partitions where the product bills them, and DNS only for the queries that
  aren't cached (`query_ratio`). Problem budgets moved with their references; every verdict stays the same except
  Web Crawler's first-try design, which now fits the budget on the normal day (it still fails the hidden tests).
- **The live cost matches Results** once a run ends, and both say which test's load and which prices they use. Test
  results show each test's availability limit and any lost messages; the brief shows a test's own availability limit.
- The Guide shows prices per shard and per write where a product has them, and says how the simulator bills it.
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
