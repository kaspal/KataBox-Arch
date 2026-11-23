# Known issues and limits

What KataBox Arch doesn't do yet, or does in a simplified way. The simulator's own simplifications are listed in
[ADR 0006](adr/0006-simulation-model.md); each problem's editorial has a "What the simulator leaves out" section where
it matters.

## Not built yet
- **AI review, Explain my design and the mock interview** arrive in M4. The buttons say so; the AI will never change
  a score.
- **GIF export** is planned; WebM video, images (SVG, PNG) and Mermaid work.
- **PDF printing** works on Windows only for now. On Linux, use **Save print-ready HTML** and print it from a browser.
- **The print layout** leaves out the concept cards, the simulator tables and the quizzes (they need the app).
- **WebM records in real time** (a 30-second clip takes about 30 seconds) and stops if the window is hidden. Web
  views without `MediaRecorder` (WebKitGTK on Linux) can't record it; the menu says so.
- **Saved WebM files carry no duration:** browsers' `MediaRecorder` doesn't write one, so some players show the
  length as unknown and seek roughly. The video itself plays normally.
- **Exported images name their fonts but don't embed them;** another computer shows them in its own fallback fonts.
- **The in-app problem editor** is planned; authors use `ka-pack new`, `try`, `validate` and `pack`
  ([pack-authoring.md](pack-authoring.md)).
- **The Progress page** is a placeholder; progress shows on Home, the Problems list, the Guide and pattern counts and the drill pages.
- **Linux builds** are configured but not yet built or tested (they need a Linux host).

## Guide: known gaps
- **Concept cards** exist for 18 of the 25 component kinds. Microservice, Client, DNS, WAF, Metrics, Logs and Tracing
  have none: in the simulator these kinds are a hop, a traffic source or a design note, so a card would only repeat
  the Guide text. Their Guide entries are complete.

## Simplified on purpose
- **docker-compose import** reads `services`, `image`, `deploy.replicas`, `depends_on`, `links` and merge keys
  (`<<`); networks, volumes, ports, environment and profiles are ignored. Only the first YAML document is read, and
  keys that are anchors, aliases or structures are refused.
- **Packets per partition:** a stream hands out one message per partition at a time across the whole stream, so a
  single hot key can't overload one partition.
- **Batching** isn't simulated: counters and log shippers handle each message separately.
- **Per-provider isolation:** external providers aren't cards, so a separate queue and pool per provider can't be
  routed (Webhook Delivery's editorial explains).
- **State inside app servers** (in-memory sessions) isn't simulated.
- **DNS caching:** every request is counted as a DNS query, so designs with a DNS card cost more than they would.
- **Request-billed services** (Kafka, pub/sub, SQS, S3, DynamoDB, DNS) dominate some Set 2 budgets; the figures follow
  the registry's per-request prices.
- **Fonts in PNG exports** fall back to system fonts (an SVG drawn as an image can't load the app's fonts).

## Packs
- Packs are **unsigned**. The import preview says so and shows the SHA-256; install packs only from people you trust.
- Pack tests are limited to 120 s each, so a few built-in problems (with 150 s tests) can't be repacked as they are.
