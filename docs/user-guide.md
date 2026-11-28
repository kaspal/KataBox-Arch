# KataBox Arch user guide

KataBox Arch is a desktop app for practising system design. You place real-product components (PostgreSQL, Redis, Kafka, NGINX and many more) on a free-form canvas, wire them together, and run a traffic simulation that animates every request. When you work on a problem, the app grades your design against the problem's targets.

Everything runs on your computer, offline. This guide uses the English names of buttons and tabs; the Spanish version is `user-guide.es.md`.

## Getting started

1. **Open the app.** You land on **Home**. The first time, Home offers a tour: **Start the tour** plays "Hello, Traffic", a short sample that shows the canvas, a live run and the metrics in about a minute and a half. **Not now** hides the offer; the tour stays in **Samples**.
2. **Watch a sample.** Open **Samples** in the left rail and pick one, for example "URL Shortener at scale". Step through it with the **Next** button. Each step loads a design, runs traffic on it and shows what changed. On the last step, **Try it yourself** copies the design into the Sandbox so you can change it.
3. **Solve your first problem.** Open **Problems** and pick **#1 URL Shortener** (Easy). The canvas opens with the **Brief** tab on the right. Read the targets, then build a first design: drag a **Client**, a **Load Balancer**, an **App Server** and a **SQL Database** from the **Library** onto the canvas and connect them in that order.
4. **Run it.** Pick a test in the test picker and press **Run**. Watch the requests flow, then open the **Metrics** tab.
5. **Submit it.** Press **Submit**. The **Results** show a verdict for every test, including the hidden ones, and a score out of 100. If a test failed, **Replay at failure** shows you the moment it went wrong. Fix the design (the hints help) and submit again.

## Finding your way around

The left rail has the main pages: **Home**, **Problems**, **Sandbox**, **Guide**, **Samples**, **Progress**, and **Settings** at the bottom.

Some pages are not in the rail. You reach them from other pages or from the command palette:

- **Patterns** and **Glossary**: links at the top of the Guide.
- **Which one do I need?** (decision guides): on the Guide page.
- **Review**, **Learning paths**, **Numbers cheat sheet**, **Estimation drills** and **Find the flaw**: in the command palette.

**The command palette.** Press **Ctrl+K** anywhere. Type part of a name to jump to a page, create, open or save a design, open a recent file, open a Guide entry or a pattern, add a component to the canvas, or change the theme or language. Matching ignores accents and punctuation.

## Home

Home is your desk. From top to bottom:

- **New design** and **Open…** start a blank design or open a `.kbarch` file.
- **Practice** shows up to three cards:
  - **Due for review**: how many missed quiz questions are due today (see Review).
  - **Continue**: the last problem you worked on.
  - **Recommended next**: if you follow a learning path, its next unfinished step, with how far along you are ("2 of 9 done"). Otherwise, the next problem you haven't solved.
- Your streak ("3-day streak") sits under the cards.
- **Watch how it works**: a gallery of samples, with **All samples** to see the full list.
- **Recent designs**: files you opened or saved. **Open** reopens one; **Forget** removes it from the list (the file itself is not touched). A file that was moved or deleted shows "File not found".

## Problems

**Problems** lists 85 built-in problems in three tracks. **Core** has 45 in three sets: #1 to #15 (Set 1, from URL Shortener to Online Judge), #16 to #30 (Set 2, from Image Thumbnails to Order-Matching Exchange) and #31 to #45 (Set 3, from Weather API Proxy to Payment Fraud Scoring). **AI systems** and **Data engineering** have 20 each. Each row shows the number, the title, what the problem teaches, the difficulty, your status (**Solved**, **Attempted** or **Not started**), your best score, and how many estimation drills you have done for it.

To narrow the list:

- **Search problems** matches titles, what they teach and their topics, in English and Spanish.
- The difficulty buttons: **All**, **Easy**, **Medium**, **Hard**.
- The topic chips (for example `#caching`): click one to filter, click it again to clear.
- **Source** (**All**, **Built-in**, **Packs**) appears once you have installed a problem pack. Problems from a pack carry a badge: "From the pack <name>".

At the top of the page:

- **Continue: <title>** reopens the problem you worked on last.
- **Random problem** opens a problem you haven't solved yet.
- **Import pack…** adds a problem pack (see Problem packs).

**Cost challenges.** Some problems have a second goal, "same SLO, half the cost" (or another share): pass every test
within a lower monthly budget than the reference. The challenge opens once you have solved the problem; the
**Cost challenge** filter lists the problems that have one, and **Submit as the cost challenge** judges your design
against it. Each challenge is proven by a design that meets it, and only targets of 80% of the reference's cost or
less are offered.

**Outage replays** (in the command palette and the Incident response path) are 20 well-known kinds of incident, told
plainly: watch the design as it was fail, then **Fix it** so it passes the incident, a normal day and the aftershock.
Each replay is a composite of a common failure, not a record of one company's outage.

## Solving a problem

A problem opens in the same layout as the Sandbox, with a **Brief** tab in the side panel. Your design is saved automatically as a draft while you work, so you can leave and come back. You don't need to save a file unless you want one.

### The brief

The **Brief** tab holds everything the problem tells you:

- **The statement**: what you are building and what matters.
- **Targets**: **Peak traffic**, **p95 latency**, **p99 latency** (only on problems that set one), **Availability** and **Budget** per month. The **Numbers cheat sheet** link sits next to them.
- **Traffic**: the kinds of request and their share of the load.
- **External providers**, on problems that call one (for example a payment provider): its median and p99 latency and its rate limit.
- **Tests**: every test by name and length. Some are marked **hidden**: you see their names, but not their traffic or failures, until you solve the problem.
- **Estimate first**: the problem's back-of-the-envelope drills, folded until you open it, with a count such as "1 of 2 estimated". These never affect your score.
- **Related in KataBox**: KataBox Learn topics and problems that practise the algorithms behind this one. KataBox is a separate app, so these are titles to look up there, not links.
- **Hints**: three per problem, unlocked one at a time with **Show hint 1**, **Show hint 2** and **Show hint 3**. Each one gives away a little more.
- **Editorial**: the worked solution. It appears once you have solved the problem.

### Run and Submit

**Run** plays one test live on the canvas. Choose it in the test picker next to Run. The picker lists the visible tests; hidden ones only appear there when you replay them from the Results.

**Submit** judges the design on every test, hidden ones included. While it works, the button says **Judging…**. Then the **Results** open:

- A summary, such as "3 of 4 tests passed; the design costs $1,840 a month."
- **Score**, split into **Correctness**, **Cost** (yours against the reference's) and **Latency headroom**, plus your best score on this problem.
- Each test with its verdict, its p95 (and p99 where the problem sets one), error rate and availability, and, for a failed test, the bottleneck: for example "PostgreSQL saturated from 34 s." or "Redis was down from 30 s."
- **Replay at failure** starts that test's live run about two seconds before the bottleneck appeared, with the failing card selected. This works for hidden tests too.
- **Stuck?** shows your latest hint, or unlocks the next one.
- Once the problem is solved, **Compare with reference** is here too.

The first time you pass every test, the Results say "Solved! The editorial is now in the Brief tab."

### After you solve it

The Brief gains a **Compare** section:

- **Compare with reference** opens your design next to the reference design. **Side by side** shows them in two panes that zoom and pan together; **Overlay** draws the reference faintly behind yours.
- A chart plots your accepted submissions by cost per month against p95 latency, next to the reference. Lower left is better: cheaper and faster.

## How grading works

The judge is deterministic: the same design always gets the same verdict and score, on every machine. Each test runs with its own fixed seed, so a run never gets lucky or unlucky.

Before any test runs, the design is checked: the wiring rules, a path for every kind of request, and any component the problem requires. If something is missing, the whole result is **Invalid design**, with the reasons listed, and the score is 0.

Each test then gets one verdict. The first that applies wins:

1. **Invalid design**: the design breaks a rule.
2. **Dropped requests**: too many requests failed, or messages were lost.
3. **SLO breach**: p95 latency over the target, availability under the target, or too many stale reads.
4. **Over budget**: the monthly cost (measured on the first test) is over the budget.
5. **Accepted**: everything held.

The problem is solved when every test is **Accepted**.

The score is out of 100:

| Part | Points | How it is earned |
| --- | --- | --- |
| Correctness | 70 | In proportion to the tests accepted: 3 of 4 gives 52. |
| Cost | 20 | Full marks if you cost no more than the reference; less the more you spend over it. Scaled by the share of tests accepted. |
| Latency headroom | 10 | How far under the p95 target your accepted tests stay. Half the target or less earns all 10. |

A design that passes nothing earns no cost or latency points, so a cheap broken design doesn't score.

**Hidden tests** model the things that go wrong in production: a spike, a node that dies, a hot key. The Results show their names, verdicts and metrics, never their traffic or failure schedule.

## The Sandbox

The **Sandbox** is a free canvas with no problem attached. It has three areas: the **Library** on the left, the canvas in the middle, and the side panel on the right with the **Inspect** and **Metrics** tabs (and **Brief** when you are solving a problem).

### Adding components

The **Library** groups components into **Clients & Edge**, **Compute**, **Data**, **Async** and **Ops**, followed by **Zones & notes**. **Search components** filters the list.

- Drag an item onto the canvas, or focus it and press **Enter** to add it in the middle of the view.
- If a component comes in several products, a picker asks "Which Cache?" (for example **Redis** or **Memcached**). You can change the product later in the inspector.
- Each item has an **About** button with a short description, its flavours, what to watch for, and **Open in Guide**.

### Wiring

Drag from one card to another to connect them. The app checks the wiring rules as you drag and explains a refusal, for example:

- "A client can't reach a data store directly. Put a service in between."
- "Messages leave a queue or stream only through a consumer service."
- "Data stores answer queries; they don't start calls."

When two cards can be joined in more than one way, a **Connect with** picker asks for the protocol: **HTTP**, **gRPC**, **WebSocket**, **Query**, **Publish**, **Consume** or **Replicate**.

From the keyboard: select one card and press **C**. Pick a target with the arrow keys or **Tab**, then press **Enter**. **Esc** cancels.

### The inspector

Select a card, a connection or a zone to edit it in the **Inspect** tab. For a card you can change the **Label**, the **Flavour** (the product) and its **Properties**: instances, service time, read replicas, TTL, memory and so on, depending on the kind.

Some properties carry a tag:

- **cited**: a calibrated default taken from a cited source.
- **ballpark**: a calibrated default, a conservative figure whose source is still being confirmed.

Untagged values are design choices for you to set. A property you have changed shows a reset button that puts it back to its default. **Learn more** opens the component's Guide entry at its "In the simulator" section.

With nothing selected, the inspector shows the **Design name** and counts of components, connections, zones and notes.

### Zones and notes

- **Region** and **Availability zone** frames group cards. Cards inside a zone move with it, and an availability zone can sit inside a region.
- **Autoscaling** (in the Compute group) adds an **Autoscaling group** frame. Compute cards inside it scale between the **Minimum instances** and **Maximum instances**, adding instances when utilisation stays above the **Target utilisation**. Set the **Scaling policy** in the inspector.
- **Sticky note**: double-click it to write.

### Editing

- **Undo** (Ctrl+Z) and **Redo** (Ctrl+Y or Ctrl+Shift+Z) are in the toolbar.
- **Delete** or **Backspace** removes the selection.
- **Snap to grid** snaps cards to an 8 px grid. Hold **Alt** while dragging to place a card freely. You can also set snapping in Settings.
- The zoom buttons and **Fit the design** sit in the corner of the canvas, with a minimap.

## Live runs

A live run animates every request through your design, in simulated time.

**Starting a run.** In the Sandbox, choose the **Traffic** preset (**Normal traffic**, **Spike ×4** or **Kill a node at 20 s**) and press **Run**. On a problem, choose a test instead. When the run ends, **Run again** starts it once more.

**The timeline** under the canvas has **Pause** and **Resume**, the simulated clock ("12.0 s of 60 s"), a scrubber, and the **Speed** (0.25× to 8×). Diamonds on the scrubber mark failures, with a tooltip such as "A card dies at 20 s". Dragging the scrubber jumps to that moment and pauses there; because the run is deterministic, you see exactly what happened.

**Inject failures.** While a run is playing, select a card and open **Inject**:

- **Kill it**
- **Slow it down (+200 ms for 10 s)**
- **Flush it (empty the cache)**, for a Cache or CDN card
- **Triple the traffic for 10 s** (needs no selected card)

**Stop** ends the run. If you edit the design during a run, the app starts a fresh run of the edited design with the same seed and traffic.

**The Metrics tab** shows **p95 latency**, **Throughput**, **Error rate** and **Cost** per month for the last simulated second, plus a p95 trend line and the **Busiest components**. Some designs also show **p99 latency**, **Availability** or **Stale reads**. On heavy traffic, the simulator follows only a share of the requests and scales the counts back up; the tab says so when it does.

**Find breaking point** pushes the traffic up in 20-second runs until the design breaks, meaning p95 above 200 ms or more than 1% errors. It reports how much it holds ("Holds about 9,600 req/s (2.4× the clients' traffic)") and which card gives way first. If you change the design afterwards, run it again.

## Files and export

Designs are saved as `.kbarch` files. The file actions are in the toolbar, in the command palette and on the keyboard:

- **New design** (Ctrl+N), **Open design** (Ctrl+O), **Save** (Ctrl+S), **Save as…** (Ctrl+Shift+S).
- The file name sits next to the design name; a dot marks unsaved changes.
- If you close the window or open another design with unsaved changes, the app asks **Save your changes?** with **Save**, **Don't save** and **Cancel**.

**Import docker-compose…** (toolbar or command palette) turns a `docker-compose.yml` into a new, unsaved design:

- Services become cards by their image: PostgreSQL, MySQL/MariaDB, Redis/Valkey, Memcached, NGINX, HAProxy, Kafka/Redpanda, RabbitMQ, Elasticsearch, OpenSearch, MongoDB and Cassandra/Scylla. Anything else becomes an App Server, and the summary says which ones were guessed.
- `deploy.replicas` sets the card's instances (at most 64; a larger number is clamped and listed).
- `depends_on` and `links` become connections, with a protocol the wiring rules allow. A dependency the rules don't allow, or one on a service that isn't in the file, is listed instead of drawn.
- Files over 1 MB, with more than 200 services, or with YAML built to blow up (too many aliases or levels) are refused.

The **Export** menu in the toolbar turns the design into:

- **Copy as Mermaid** and **Save as Mermaid…**: a Mermaid diagram you can paste into Markdown documents.
- **Save as SVG image…** and **Save as PNG image…**: a picture of the design.
- **Save animation (WebM)…**: a video of the run, with moving requests, each card's load and a line with the time,
  p95, throughput and errors. Pick a start and a length (up to 60 s); it records in real time, so keep the window open.
  It shows the Sandbox's traffic, or the problem test Run is playing.
- In **Results**, a failed test has **Export replay** next to Replay: the same video of the part Replay shows, hidden
  tests included (nothing before Replay's start is ever shown).

## Learning: the Guide, patterns and glossary

### The Component Guide

**Guide** has an entry for every component kind, with a count of how many you have read. **Start here** lists the six pieces almost every design uses. Search with words such as "cache", "fan-out" or "replication".

An entry covers **What it does**, **Use it when**, **Skip it when**, **In the simulator** (the simulator's own figures for each product), **Common pitfalls**, **Trade-offs**, **The products compared**, **Practice it** (problems that use it), **Works with**, an **Interview tip** and **Sources**. Many entries have a **Concept card**: a short animated sample, started with **Play the concept card**. **Mark as read** records the entry as read.

Each entry ends with a **Quick check** of three questions. A missed question comes back for review (see Review).

You can reach the Guide from the canvas too: **Open in Guide** in the Library's About panel, and **Learn more** in the inspector.

**Print** (on the Guide, on each entry and pattern, and on each learning path) saves a paper copy: **Save as PDF…** (Windows) or **Save print-ready HTML…** (everywhere; open it in a browser to print it). The whole Guide prints one entry per page, then the glossary; a pattern includes its template's picture; a path lists what it covers, with its Guide entries and patterns. If a PDF can't be made, the app offers the HTML instead.

### Patterns

**Patterns** are proven shapes for common problems: Backpressure, Cache-aside, Circuit breaker, CQRS, Fan-out, Transactional outbox, Rate limiting, Read replicas, Saga and Sharding. Each explains **The problem** and **How it works**, shows **The template**, and ends with a quick check. **Insert into the Sandbox** adds the template's cards to the design you are editing, in the middle of the view; one undo removes them.

### Decision guides

Under **Which one do I need?** on the Guide page you'll find four short question flows: Cache or CDN?, Load balancer or API gateway?, Queue, stream or pub/sub?, and SQL or NoSQL? Answer one question at a time and the guide ends at the Guide entry to reach for. **Start again** resets it.

### Glossary

The **Glossary** defines the terms the app uses, such as p95, and you can search it. In statements, hints, editorials, Guide entries and patterns, the first use of each glossary term is underlined with dots: hover over it (or focus it with the keyboard) to see the definition, or click it to open the Glossary at that term.

## Practice: samples, drills and paths

### Samples

**Samples** are guided stories, each a few steps long: Hello, Traffic (the first-run tour), URL Shortener at scale, News Feed: fan-out, Chat with WebSockets and Flash Sale. Each step loads a design, runs it and shows the numbers that changed against a target. Use **Previous**, **Next** and the step dots to move, and **Replay step** to watch a step again. **Try it yourself** copies the step's design into the Sandbox as "<title> (my copy)".

### Numbers cheat sheet

Rough speeds and sizes for estimates: memory, storage and databases, network, servers, sizes, time and traffic. These are orders of magnitude, not benchmarks. Figures marked as the simulator's own are the ones the simulator uses, so your estimates and its runs agree.

### Estimation drills

Back-of-the-envelope practice for each problem. Pick a **Problem** (or press **Random drill**), work out the number, type it in **Your answer** and press **Check**. Anything within the drill's band counts; otherwise the app tells you if you are too high, too low, or more than ten times off. A worked solution shows one way to get there. The same drills appear in each problem's **Estimate first** section.

### Find the flaw

Ten designs that look fine and pass a normal day, but each has one flaw a test exposes. Open one, run it, find the flaw, fix it and submit until every test passes. Once you fix it, the explanation and the reference fix appear in the brief.

### Learning paths

**Learning paths** are eight ordered tracks: Foundations, Async systems, Data at scale, Reliability, Real-time, Incident response, AI systems and Data engineering. Each mixes Guide entries, concept cards, patterns, problems, drills and quizzes. Progress comes from what you have already done anywhere in the app. Press **Follow this path** and Home's **Recommended next** card shows the path's next step.

## Review

Every Guide entry and pattern ends with three quiz questions. A question you answer right the first time never comes back. A question you miss enters a short review queue:

- It is due the next day.
- Each time you get it right in review, it comes back later: after 1, then 3, then 7, then 21 days. Right again after the 21-day gap, and it leaves the queue.
- If you get it wrong in review, it starts over and is due the next day.

When questions are due, Home shows a **Due for review** card. Open it, or open **Review** from the command palette, and work through the questions one by one with **Next question** and **Finish**. Questions with the most misses come first. On a day with nothing due, the page says "Nothing to review today".

## Progress

The **Progress** page is still a placeholder ("Nothing to show yet"). For now, your progress shows up here:

- **Problems**: status and best score for every problem.
- **Home**: your streak and recommendations.
- **Guide** and **Patterns**: how many pages you have read.
- **Learning paths**, **Estimation drills** and **Find the flaw**: their own progress counts.

## Settings

**Appearance**

- **Theme**: System, Light or Dark.
- **Contrast**: Normal or High. High contrast uses stronger text, borders and focus rings.
- **Motion**: Follow system or Reduced. Reduced cuts animations everywhere.

**Language**: **App language**, English or Español. All content (problems, hints, Guide, samples) is available in both.

**Canvas**: **Snap to grid**, On or Off.

**Problem packs**: the installed packs with **Uninstall**, **Import pack…**, and the checkbox **Recommend problems from packs on Home**.

**About**: the app version and the privacy statement.

## Problem packs

A problem pack is a set of extra problems someone shared as a `.kbpack` file: a teacher's set for a class, a team's interview practice, a community collection. A pack contains only text and designs, never code.

**Importing.** Press **Import pack…** on the Problems page or in Settings and choose the file. The app checks the pack before anything is installed ("Checking the pack…"); a large pack can take a few seconds. The checks:

- Every file is listed in the pack with a SHA-256 checksum, and every checksum must match. Unknown, missing or out-of-place files are refused.
- File names, sizes, file types and the number of entries are limited, so a pack can't write outside its own folder or fill your disk.
- Text containing scripts, embedded frames or similar markup is refused. Pack text never loads images or follows links.
- Every problem is loaded and validated like a built-in one, and judged: its reference design must pass every test, and its naive design must fail a hidden test.

If the pack passes, the preview **Install this pack?** shows its name, author, version, licence, number of problems and SHA-256, and what will happen (a new pack, an upgrade, or a reinstall). It also says "Unsigned pack, from <author> (as stated in the pack). Only install packs from people you trust." Packs are not signed; the author's name is whatever the pack says. If the person who shared it published the SHA-256, compare it with the one in the preview. Nothing is installed until you press **Install**.

**Versions.** A pack can only be replaced by a newer version. An older version, or the same version with different contents, is refused.

**Uninstalling.** In Settings, **Uninstall** removes the pack's problems from the catalogue. Your submissions stay in your history.

Problems from packs carry a badge and can be filtered with **Source**. Home only recommends them if you turn on **Recommend problems from packs on Home**.

## Privacy

KataBox Arch works offline. It sends no telemetry, ever; Settings › About says so too. Your designs, progress and settings stay on your computer.

The toolbar shows an **AI review** button that says it arrives in M4. When it comes, it will explain your results and challenge your design, using an AI provider you choose and configure, and only when you start a review yourself. It will never change a score: grading stays with the deterministic judge.

## Keyboard shortcuts

| Shortcut | Action |
| --- | --- |
| Ctrl+K | Open or close the command palette |
| Ctrl+N | New design |
| Ctrl+O | Open a design |
| Ctrl+S | Save |
| Ctrl+Shift+S | Save as |
| Ctrl+Z | Undo (Sandbox and problems) |
| Ctrl+Y or Ctrl+Shift+Z | Redo (Sandbox and problems) |
| Delete or Backspace | Delete the selection on the canvas |
| C | Start connecting from the selected card |
| Arrow keys or Tab, then Enter | Pick the target of a connection, then connect |
| Esc | Cancel a connection or close a picker |
| Alt (while dragging) | Place a card freely, without snapping |
| Enter (on a Library item) | Add it to the canvas |

A **Skip to content** link appears when you press Tab at the top of a page.

## Troubleshooting

**"This design was saved by a newer version of KataBox Arch."** The file comes from a later version of the app. Update KataBox Arch to open it. Older files open normally; the app upgrades them as it reads them.

**"This is not a valid KataBox Arch design file."** or **"The file is too large to open."** The file is damaged, isn't a `.kbarch` design, or is over the 8 MB limit.

**"The design breaks a rule (wiring, ids or values), so it was not saved."** Look for a connection the wiring rules don't allow or a property outside its range; the inspector shows the allowed range when you type a value.

**"No request can get through this design."** Connect a Client to a service, and check that every kind of request in the brief can reach the components it needs.

**A pack was not installed.** "This pack failed its safety and content checks" means one of the checks failed; the dialog lists why under **Why**. "This pack can't replace the installed one" means it is older than the one you have, or the same version with different contents. Ask the author for a corrected or newer pack.

**A recent design says "File not found".** It was moved or deleted. Use **Forget** to remove it from the list and **Open…** to find it again.

**Where your data lives.** Your progress, settings, recent files and installed packs are in the app data folder. On Windows that is `%APPDATA%\dev.katabox.arch`. Your `.kbarch` designs live wherever you saved them. Development builds of the app show a **DEV DATA** badge and keep their data in a separate folder ending in `-dev`, so they never touch your real profile.
