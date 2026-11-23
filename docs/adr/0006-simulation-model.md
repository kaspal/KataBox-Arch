# ADR 0006: The simulation model (`ka-sim`)

- Status: accepted (M1.1, 2026-11-22)

## Context

The simulator is the product's core: it animates every request and decides every verdict. It must be simple
enough to explain in a tooltip, rich enough to reward real design moves, fast enough to judge a problem in well
under two seconds, and **deterministic**: the same design, scenario and seed give byte-identical results on every
machine.

## Decision

### Inputs and outputs
- **Pipeline:** `compile(design, registry, scenario, options)` builds an immutable graph with every route
  precomputed, or returns a lint error (`NoPath`, which the judge reports as Invalid Design).
- **Engine:** `Engine::new(compiled, seed)` runs it; `Engine: Clone` is the snapshot used for scrubbing.
- **Results:** `result()` returns a `RunResult`:
  - per run: counts weighted back to real traffic, measured/response/completion percentiles, error rate,
    availability, lost and dead-lettered messages, and cost per month;
  - per node: utilisation, queue, drops, and per-second series;
  - per system: per-second series.

### Time and events
- **Time** is integer microseconds.
- **Ordering:** events live in a binary heap ordered by `(time, sequence)`, so the order is total.
- **The events:**
  - arrivals (a Poisson process per source and request type, thinned by the traffic curve);
  - arrival at a node;
  - service done;
  - call answered;
  - call failed;
  - client timeout (5 s);
  - message re-queue (retry backoff);
  - per-second ticks.

### Routes: the caller rule
A request type's `route` is a list of steps, `role[:qualifier…][?]`, with `a|b` alternation.
- **Roles:**
  - `edge`: DNS, CDN, WAF, LB, gateway, rate limiter;
  - `compute`: app, microservice, serverless;
  - `worker`;
  - `cache`;
  - `datastore`: SQL, NoSQL, object, search;
  - `broker`: queue, stream, pub/sub, DLQ;
  - `external:<name>`.
- **Qualifiers:** `write` (the cluster primary), `ws` (the client's hop is a WebSocket), and a kind id
  (`datastore:object`).
- **Hops, then calls.** Steps before the first caller step are **hops**: any number of edge nodes along
  HTTP/gRPC/WebSocket edges, or at least one if the step isn't optional. From the entry caller on, the steps are
  **calls made by that caller, in order**, until a `broker` step.
- **Brokers:** the caller publishes and is released. The **consumer** on the broker's Consume edge (the next step)
  becomes the caller of the steps after it. Data stores never call onward.
- **Cache-aside:** `cache?` is cache-aside. A hit ends the request; a miss falls through to the next data step, and
  its answer fills the cache. `cache:write` writes the cache as a store, and `cache` without `?` reads it as one.
- **Alternation:** `compute|datastore:object` lets a CDN's origin be either.
- **Choosing hops:** routing is compiled once. A BFS finds, per request type, the next hops from every node that
  can still reach a valid entry, and the call targets of every valid caller. A request type with no complete path
  is a lint error that names it.
- **Sources:** client cards share a type's load by their `rps` prop. Scheduler-sourced types start at a scheduler
  card, which is the first caller.

### Servers
- **Capacity:**
  - a card has `servers = instances × concurrency` (the flavour's `SimDefaults` when it has no such prop; ADR 0005),
    split into **lanes**
    for NoSQL partitions and SQL/search shards (a key always lands in the same lane, so hot keys make hot shards);
  - each lane has a bounded queue, and a full queue drops the request.
- **Service time:** lognormal from the `service_ms` median and a per-kind p99/median ratio. SQL/NoSQL times scale
  with the payload (`× (1 + payload_kb/64)`), and async work scales with its fanout units.
- **Held slots:** compute and workers keep their slot until all their calls answer (thread-per-request), so a slow
  database ties up the app tier. They release it when they publish to a broker, and when the client's 5 s timeout
  fires (the remaining work is cancelled lazily).
- **Clusters:** SQL/NoSQL/cache/search cards joined by Replicate edges form a cluster.
  - The root of the Replicate edges is the primary; the other members follow in id order.
  - A Query edge to any member reaches the cluster.
  - Writes go to the primary; reads spread over healthy members.
  - Failover (M1.5) promotes the next member in id order after `failover_s`.
- **Network latency per hop:** a protocol base (HTTP 0.5 ms, gRPC/WebSocket 0.3, query 0.4, publish/consume 1.0)
  plus the zone distance (same AZ +0.2 ms, cross-AZ +1.5, cross-region +70). A response pays its hop chain again
  on the way back.

### Kinds with special behaviour
- **Cache and CDN:** keys are ranked by Zipf popularity over the request type's `key_space`.
  - A key ranked beyond the cache's capacity (`memory_gb / value_kb`) always misses.
  - Tracked keys (up to 200k ranks) hit if they were filled within the TTL.
  - Colder keys within capacity hit with the analytic probability `1 − e^(−rate·pmf·TTL)`.
  - Caches start warm, with staggered ages, unless the scenario says cold.
  - A CDN has unlimited capacity and serves only `cacheable` types.
- **Rate limiter:** token buckets per API key. Shed abusive traffic is success; shed legitimate traffic is an error.
- **Serverless:** a cold start is paid when a request needs a new warm instance.
- **Brokers:**
  - a publish is acknowledged after the hop and the broker's median publish latency (`service_ms`);
  - queues dispatch to any free consumer slot; the reserved slot is the consumer's alone, and the consumer's own calls
    queue for their servers like any request;
  - streams cap in-flight messages at `partitions`;
  - pub/sub pushes one copy per subscriber, and the message completes when all are done.
  - A failed message is retried with exponential backoff up to `max_retries`, then dead-lettered if a DLQ is wired
    (`queue → dlq` Publish), otherwise **lost**.
- **External providers** are scenario entries, not cards. They have a lognormal latency and an optional rate limit;
  chaos can make one fail (1 s connect timeout) or slow.

### Chaos, health checks, failover, autoscaling (M1.5)
- **Scheduling.** Chaos events come from the scenario: each starts at `at` and, with `for`, ends `for` seconds later.
  They are scheduled only when a run has them, so runs without chaos keep their exact event order.
- **The events:**
  - **kill:** the card stops answering. Queued work fails now, work in service fails when it ends, and a caller
    waiting on a dead card fails when the call returns. A cache comes back empty.
  - **partition:** every card in a Region or AZ (by id or label) is killed for the duration.
  - **latency:** adds `add_ms` to each service time on the card.
  - **flush:** empties a cache or CDN. Tracked keys are cleared, and the analytic tail's window restarts at the
    flush.
  - **traffic:** multiplies arrivals.
  - **external:** a provider fails (1 s connect timeout) or slows.
- **Selectors:** an id, or a kind with a cluster role (the current primary, a replica, or any member). The first in
  id order, or every match with `all`.
- **Health checks** run every 2 s; a change is believed after two checks in a row.
  - Until then, routing still sends requests to a dead card, and they fail. Afterwards, load balancers, callers and
    replica reads avoid it.
  - A cache-aside step whose caches are all known to be dead is skipped: a miss, read from the store.
  - A revived card rejoins after it is seen healthy.
- **Failover:** when a cluster primary dies, the next live member in id order is promoted after the primary's
  `failover_s`. Writes fail until then.
- **Autoscaling:**
  - every 5 s, each wrapper looks at its cards' mean utilisation since the last tick;
  - above target, it scales out to bring utilisation back to the target; below half the target, it scales in by
    one;
  - each decision applies after `scale_delay_s`, within min and max;
  - servers (and the sampling stretch) change at runtime;
  - cost uses the time-averaged instance count.

### Stale reads (M1.7)
- **Keys are shared across request types:** rank k is the same item for reads and writes.
- **Writes.** Each tracked key (up to 200k ranks) remembers its last write: a store write, or a `cache:write`.
- **A read counts once:** a cache hit, or the store read after a miss. It is stale when:
  - **a replica lags:** it is served by a replica (not the current primary) within that replica's
    `replication_lag_ms` of the key's last write;
  - **a cached copy is old:** a cache-aside hit whose copy was filled before the key's last write. Cache-aside
    doesn't invalidate, and that is the lesson.
- **Write-through** (`cache:write`) refills the key for every request type, so its readers see the new value.
- **Untracked keys** (the analytic tail) are never counted stale. This is a documented undercount.
- **The metric:** `stale_rate` = stale reads / reads, per run. Live frames carry it, the Metrics tab shows it, and the
  breaking point reports it at the limit. Problems may set `pass.stale_rate`.

### The breaking point (M1.6)
- **`breaking::find_limit`:**
  - runs the scenario's mix in 20 s windows at traffic multipliers ×0.25, ×0.375, … (growing ×1.5);
  - stops at the first window whose p95 passes the target or whose error rate passes the limit;
  - bisects to within 5%;
  - reports the last passing rate.
- **The first failure** is the first card to give way in the first failing window: the earliest second in which a
  card is at least 95% busy or drops, with ties going to the busiest, then the lowest id.
- **The ceiling** is ×1024; a design that never fails up to there reports "at least".
- **Deterministic** per seed (tested).
- **The Sandbox** uses p95 200 ms and 1% errors; problems will use their own targets.

### Sampling (the scale trick)
Above `sample_cap = 5000` simulated arrivals per second, only a share `f = cap / rate` of requests is simulated.
- **Weights:** every count is weighted by `1/f`.
- **Capacity:** each lane keeps `servers_eff = max(1, round(c·f))` servers, with service times stretched by
  `servers_eff / (c·f)`, so capacity is preserved exactly.
- **Floor (review A1):** every lane with `c ≥ 2` keeps at least 4 effective servers:
  `f = max(cap/rate, max over lanes of min(1, 4/c))`. Single-server lanes (a Redis event loop) are unaffected.
- **Accuracy rule:** a sampled run must agree with a full unsampled run at peak load. p95 and utilisation must be
  within ±10%, with the same verdict. If a case fails, its `sample_cap` is raised and recorded. The check runs at
  every milestone gate; the results are in `docs/sim-stability.md`.

### Cost
A teaching price model per flavour (`registry::Cost`: per instance-month, per GB-month, per million requests).
Monthly cost = instances × price × shard factor + memory × GB price + mean real rps through the card ×
per-request price. It is consistent across designs and not a vendor quote.

**Cost per shard (M5.0).** SQL and Search are billed per shard and NoSQL per partition. The shard factor is the
card's shard (or partition) count over its flavour's default count: a card at its default costs exactly what it did
before, more shards cost more, and fewer cost less, so the cost score rewards a design that doesn't over-shard.
Replicas stay separate cards with their own price.

**Billing calibration (M7b).** The products the M7b plan review cited are billed at their list prices (us-east-1,
on demand, read 2026-11-22; `docs/calibration.md` quotes each), with no free tiers, discounts or data transfer:
- `Cost` gains `per_lane_month` (a price per shard or partition), `per_million_write` (writes, publishes and
  messages priced apart from reads; unset bills them as reads) and `unit_kb` (a request counts
  `max(1, payload_kb / unit_kb)` billing units). Flavours may now have their own price.
- Each card counts billed read and write units: a datastore call with a write is a write; a broker publish, a
  message stored and a dead letter are writes; a pub/sub delivery and the receive of a retried message are reads.
- `per_lane_month` × lanes replaces the shard factor for Kinesis (stream partitions as shards), Cassandra and MongoDB
  (partitions as nodes or shards). SQL and Search keep the M5.0 shard factor; other kinds have none. DynamoDB on
  demand and Kafka partitions are free.
- DNS bills `query_ratio` (default 0.01) of the requests through it: clients and resolvers cache answers.
- The live cost spreads billed units over `min(now, duration)`, as the judge does, so a finished run's tile equals
  the judged cost (before, the 30 s drain diluted the per-request part to 2/3).
- Verdict guard (review B1): every reference stayed Accepted on every test, every naive design still fails a hidden
  test, every flaw and sample still holds. Budgets whose reference cost moved were re-tuned at their old ratio to the
  reference. One visible naive verdict moved: Web Crawler's naive design is no longer Over Budget on Normal.

### M5.0 additions (the M1–M3 review's Q2 decisions)
Each addition has tests in `tests/additions.rs`. Golden c changed with the DynamoDB write factor, and golden d is
new.
- **Worker leases.** A queue or stream message whose consumer dies (while working on it, waiting on its calls, or
  on its way to it) is not failed at once. It comes back when its lease runs out: a queue's `visibility_timeout_s`,
  a stream's `session_timeout_s` (Kafka's consumer default, 45 s). It holds its broker slot until then, so a stream
  partition stays blocked through the rebalance, and the redelivery counts as an attempt: after `max_retries` it is
  dead-lettered or lost, as before. Pub/sub keeps the plain retry path.
- **Change data capture (route step `cdc`).** It must follow a `datastore:write` to SQL or NoSQL.
  - The caller's answer leaves at the commit, so the request's latency ends there. The change is published by the
    database card that committed it, to the stream on its CDC edge (`sql|nosql → stream`, Publish), chosen by key,
    so it counts only toward **completion**.
  - Only committed writes publish; a write that fails (a failover) publishes nothing.
  - A stream that is down, full or cut off doesn't lose committed changes. They wait in a per-database backlog (the
    database log), in commit order, and go out when the stream is back. Changes older than the database's
    `cdc_buffer_s` (default 300 s) count as lost. Golden d pins this.
  - Changes to one key keep their order (tested by an audit of the stream's acceptance order).
  - A promoted replica with no CDC edge of its own publishes nothing: its changes are lost (a design lesson).
  - A design whose write targets have no CDC edge to a stream with a consumer has no path (Invalid Design).
- **DynamoDB writes.** A write occupies `write_slots` read slots (3 on DynamoDB: a partition takes 3,000 reads but
  1,000 writes per second). Reads count as strongly consistent, which is conservative.
- **Key hash.** The LB algorithm `ip_hash` is now `key_hash` ("Key hash (sticky)"): it pins each request key, such as
  a user or session, to one server. `.kbarch` v3 renames the value; the simulator still reads the old one.
- **Read from the primary.** SQL `read_from = primary` sends reads to the primary only (read-your-writes); reads no
  longer spread over replicas, so none are stale.
- **Provider rate cuts.** External chaos may set `rate`: the provider's rate limit drops to that many calls per
  second for the duration.
- **Network splits.** Chaos `split { zone }` keeps the zone running but cuts every link across its border: hops,
  calls and their answers, publishes and consumer dispatch. Cards outside every zone count as outside. Routing learns
  of the split after two health checks (2–4 s) and stops sending across it. A replica cut off from its primary
  receives no writes, so reads of keys written since the split began are stale. `partition { zone }` still means the
  whole zone is lost.
- **p99 limits.** `pass.p99_ms` is judged only on runs with at least 1,000 simulated completions (`ka-pack` refuses a
  problem whose reference falls short), and the sampled-vs-full accuracy check includes p99 where a problem uses it.
- **Optional hand-offs (M5.3).** A `broker?` step (with its consumer) may be skipped: a caller wired to such a
  broker publishes as usual; a caller with no broker edge makes the next segment's calls itself, still holding its
  slot, so the answer waits for them. One route then fits both the queued design and the inline one (Find the flaw's
  `sync-email`, Set 2's Checkout).
- **Unfinished work at the end (M6.2).** Asynchronous work still waiting when the run and its 30 s drain end (a
  backlog behind consumers that died for good, changes waiting for their stream) is counted in the completion
  percentiles at its age then: a lower bound, so a stalled pipeline can't look fast just because nothing finished.
  It isn't lost (it is still in a broker). Set 1 verdicts are unchanged; two naive designs' hidden-test p95 moved by
  under 5% (online-judge "contest", web-crawler "duplicates").
- **Local reads (M6.3).** A read from a data cluster goes to a live member in the caller's own Region when there is
  one (the nearest replica), round robin among those; otherwise to any live member, as before. Writes still go to
  the primary, wherever it is. Designs without Regions are unaffected (Set 1 results are identical).
- **`@primary` zones (M6.3).** A `partition` or `split` whose zone is `@primary` hits the Region holding the current
  primary of the design's main data store (its first SQL cluster by card id, else its first NoSQL one), so a region
  test doesn't depend on how the player labels Regions. If that primary is in no Region, the zone is the whole design
  except clients, DNS and CDNs (the outside world), so leaving Regions out can't dodge the test.
- **Pub/sub and dead subscribers.** A delivery to a subscriber that is down fails and is retried with backoff from
  the topic, which holds it until a subscriber is back; it is lost only when its retries run out.
- **Still simplifications:** DNS TTL, WAF and gateway blocking, the serverless cold-start rule, sagas and circuit
  breakers.

### M8 additions (opt-in; goldens a–d unchanged)
Every M8 feature is switched on by a prop, a request-type field or a scenario field whose default is the behaviour
before M8, and its events are scheduled only when a run uses it. Tests: `tests/m8.rs`, one golden each.

- **Client retries and timeouts (S8, golden l).** Client cards gain `timeout_ms` (default 5,000: the old fixed
  client timeout), `retries` (0), `backoff_ms` (100), `jitter` (`none` | `full`) and `retry_budget_pct` (0 = no
  budget).
  - A synchronous request that fails (an error, a drop, a 429, a timeout) is tried again after
    `backoff_ms × 2^try`, times a uniform draw with full jitter (its own `retry` stream).
  - The new try is a fresh request with the same type, key and start time: the user-visible latency spans every try,
    and only the last try's failure counts as an error. Each try counts in `retried` and loads every card it reaches,
    which is how retries amplify an outage.
  - A budget allows retries only while they stay under that share of first tries over the last 10 seconds.
  - The request count (`requests`) counts users' requests, not tries.
- **Service calls, call timeouts, fallbacks, breakers, bulkheads, shedding (S8, golden l2).**
  - Route step `service` (or `service:<kind>`): the caller calls a compute card (Http/gRPC edge) that answers
    without calling onward, a leaf dependency. `service?` is an optional dependency with a fallback: when the call
    fails, times out or is refused by a breaker or bulkhead, the step is skipped and the request goes on. Every
    other failed call still fails the request.
  - App, microservice and worker cards gain `call_timeout_ms` (0 = wait), `breaker_errors_pct` (0 = no breaker),
    `breaker_open_s` (30), `bulkhead` (false) and `shed_queue` (0 = never; the gateway has it too).
  - A call timeout stops the caller waiting; the work it asked for runs on unwanted (it still loads the callee).
    Each call is numbered, so an answer that arrives after its caller gave up is ignored.
  - The breaker is per caller and dependency: once at least 20 calls (scaled by sampling) in the last 10 s have
    failed at the given share, it opens for `breaker_open_s`, failing calls at once; then it closes and counts
    afresh (no half-open probes: a still-failing dependency trips it again within a few calls).
  - A bulkhead splits the caller's slots evenly over its distinct dependencies; a call whose pool is full fails at
    once.
  - Shedding: a request that would wait behind `shed_queue` others (per lane, scaled by sampling) is dropped at
    once, an error, instead of queuing toward a timeout.
- **Deploys, config pushes, the control plane, DNS, health-check ejection, recovery (S8, golden l3).**
  - Chaos `deploy { id|kind…, at, error_rate, add_ms, for }`: the selected services run a bad build on a share of
    their instances (a request served there fails with `error_rate` and takes `add_ms` longer; one `s8` stream
    draws). The share grows by the card's `rollout_percent` every `bake_s` (`bake_s` 0: all at once); with
    `auto_rollback`, each stage is noticed one health interval (2 s) after it starts and rolled back. `for` is the
    hand rollback.
  - Chaos `config_push { at, error_rate, add_ms, for }`: every service turns faulty at once. A Config card with
    `staged` sends it one Region per stage (zone order; services outside Regions last), every `bake_s` (at least
    one health interval), halting at the first stage, which is noticed after 2 s; with `kill_switch` the change is
    rolled back 1 s after it is noticed (with or without staging).
  - Chaos `control_plane { at, for }`: no autoscaling decision applies and killed cards don't come back until it
    ends; running instances keep serving.
  - Chaos `dns_outage { at, for }`: at a DNS card, a request fails with probability min(1, elapsed / `ttl_s`):
    answers cached within the TTL still work (uniform cache ages). `cache_flush` is another name for `flush`.
  - DNS `routing = failover`: answers point at the first target in design order while it is healthy, else the
    next live one; after a switch, a share 1 − elapsed / TTL of clients still uses its cached (dead) answer.
    `round_robin` (the default) keeps the old spread over every healthy target.
  - Load balancers: `health_timeout_ms` ejects a target whose estimated queue wait (queue ÷ servers × service
    time) passes it on two checks in a row, and takes it back after two checks under it; `min_healthy_pct` keeps
    that share of targets in rotation. With every target ejected the LB answers 503. Health checks run whenever a
    design uses this, even without chaos.
  - `pass.recovered_by_s = T` (the metastable check): from second T to the end of the test, every second with
    traffic must be within the test's error limit (1% if unset) and p95 limit.
- **Quorum, fencing and races (S7, goldens k and k2).**
  - Route step `lock`: a call to a Coordination Service (`coord`, Query edge) or to a cache used as a lock. A lock
    from `coord` needs a quorum: the caller's side of any split must reach members holding more than half of the
    cluster's instances (each `coord` card counts its `instances`; Replicate edges join cards into one cluster).
    Without it the call is refused. A cache lock has no quorum. A lock is a grant, not a data write.
  - SQL and NoSQL gain `split_failover` (`manual`: as before M8, a replica takes over only when the primary dies;
    `auto`: once health checks see a split, plus the primary's `failover_s`, a member on the far side promotes
    itself for writers on that side) and `fencing`. A write landing on such a self-promoted primary is a
    `conflict`, unless the data card has `fencing` and the request holds a lock from a quorum `coord`: then the
    stale primary refuses it (an error).
  - Request type `contended = { keys, share }`: `share` of the requests check, then write, one of `keys` hot keys.
    The check is the request's first call; a write whose check came before another write's commit on the same
    key raced: a `conflict`, unless the write step is `datastore:write:atomic` (a conditional write: the loser is
    refused cleanly, an answer) or the request holds a lock.
  - `pass.conflicts` (usually 0) fails a test as Dropped Requests. Conflicts are counted exactly, so `ka-pack`
    refuses a test that limits them on a sampled run. New result fields are left out of the JSON when zero.
- **Connections (S1, golden e).**
  - A request type with `kind = "connection"` (and `share = 0`) stands for clients holding `connected`
    long-lived connections (times the traffic multiplier), reached over `ramp_s` (0: connected from the start, as
    far as capacity allows). It lands on the WebSocket gateways and IoT brokers its clients reach through edge
    hops; it isn't a Poisson source and isn't sampled.
  - A fluid model steps every 100 ms (exact counts, no per-connection events): a holder holds up to
    `instances × min(max_connections, memory_gb / conn_memory_kb)`, and accepts `accept_per_s × instances` new
    connections per second (spread over the live holders by their room). Refused attempts are `conn_drops` (and
    errors of the type) and retry after the client's `reconnect_backoff_s`, spread over its `reconnect_jitter_s`.
    When a holder dies its connections reconnect spread over `reconnect_jitter_s` (0: all in the same 100 ms, a
    reconnect storm).
  - Each connection receives `push_rps` messages per second (times the traffic multiplier, so a spike is a burst of
    updates); a holder sends `msgs_per_s × instances`, and the excess waits in a backlog. Delivery latency
    (backlog wait + 1 ms) is the type's measured latency.
  - `connected_min` is the least connected share of the target in any second from `ramp_s` + 10 s to the end;
    `pass.connected_min` fails a test as SLO Breach.
- **Token requests (S2, goldens f and f2).**
  - Route step `model` (or `model:chat|embed|classify`, matched against the card's `model_type`): a leaf call to
    a Model Inference card on an HTTP/gRPC edge. Request types may carry `tokens_in` and `tokens_out` (mean and
    p95, drawn once per request as a lognormal from the `tokens` stream). `embed` produces no output tokens and
    `classify` one.
  - Service time = `ttft_ms` + prompt ÷ `prefill_tps` (time to first token) + output ÷ decode speed. Hosted
    flavours (Claude, OpenAI, Bedrock, Gemini APIs) have no slot limit; self-hosted ones (vLLM, Triton) have
    `instances × batch_slots` slots and decode at `tokens_per_s × (1 − 0.5 × busy ÷ slots)`, measured when the
    request starts (continuous batching: more requests, each slower, more in total; the 0.5 is a modelling
    choice).
  - Hosted limits: per-minute request (`rpm`) and token (`tpm`) buckets holding one minute's allowance, refilled
    continuously (scaled by sampling). A call past them is refused with a 429 after 50 ms (`rate_limited`), a
    failed call that client retries may try again.
  - TTFT (request start to first token) is recorded per type and overall; `pass.ttft_p95_ms` fails a test as SLO
    Breach.
  - Usage meters bill tokens (`tokens_in_m`, `tokens_out_m` per million), halved with `api_mode = batch`; the
    batch API also skips the per-minute limits. Its delivery window isn't simulated. Connection-minutes and pushed
    messages (S1) bill through the same meters.
- **Semantic cache (S3, golden g).** A cache with `mode = semantic` on a `cache?` step answers a request with
  probability `semantic_repeat_share` × fill × capacity: the request type declares how many requests are
  near-repeats; fill = 1 − e^(−seen ÷ distinct), with `seen` the lookups since the cache was last emptied and
  `distinct` the type's `key_space`; capacity = min(1, entries that fit (`memory_gb ÷ value_kb`) ÷ distinct). A
  warm cache starts full; a flush or restart empties it. Each lookup adds 15 ms (an embedding and an ANN search; a
  modelling choice). The design can't raise the hit rate above what the traffic declares.
- **Jobs (S4, golden h).**
  - A request type with `kind = "job"` (share 0) and `job = { work_units, tasks, skew, starts }` runs one job at
    each start time. Its route is the calls each task makes, as its Batch card's caller (like a scheduler route).
    Runs with jobs are never sampled.
  - Task sizes are Zipf weights (`skew`; 0 = equal) shuffled by the `jobs` stream and scaled to `work_units`
    executor-seconds. A Batch card with `salting = k` splits its largest 1% of tasks k ways. Each launch draws
    noise (lognormal, median 1, p99 1.5) and adds `task_overhead_ms`.
  - A task is a request that arrives at the card and holds an executor (`instances × executors`, FIFO, no queue
    limit) for its time, then makes its calls; the job finishes with its last task.
  - `speculation`: once 95% of the tasks are done, a task running longer than 1.5 × the median finished task gets a
    copy; the first copy to finish wins. It redraws noise; it can't shrink a task that is big because of skew.
  - Chaos `spot { kind|id, at, share, for }`: `share` of the card's executors go away until `for` ends, and that
    share of its running tasks start again (counted in `tasks_retried`): from scratch, or from their last 10%
    checkpoint with `checkpoint`. A task whose call fails starts again too.
  - `jobs` reports each run's start and completion time; `pass.job_deadline_s` fails a test (SLO Breach) when a
    job finishes late or not at all. Job tests run at most an hour of simulated time (plan ★10 asked for a day;
    workloads are scaled and the brief says so).

### Determinism
- **Math:** no `f64::ln`/`exp`/`powf` (platform libm). `ka_core::detmath` implements them with exact IEEE
  operations.
- **Collections:** no `HashMap`/`HashSet` and no wall-clock time. `crates/ka-sim/clippy.toml` bans them all.
- **Random streams:** every draw comes from a named xoshiro256** stream (`ka_core::rng`): per source, per node, plus
  one shared stream. Editing one card doesn't reshuffle the others.
- **Ordering:** floats are summed in a fixed order (no parallel reductions).
- **Tests:**
  - goldens pin three designs (`tests/golden/*.json`, regenerated with `KA_UPDATE_GOLDENS=1` only on purpose);
  - the same seed gives byte-identical JSON;
  - a cloned engine finishes identically to the original.

## Consequences
- **Fast:** a 60 s run at 50k rps takes about 0.44 s in release (golden b).
- **Calibration:** the model's figures live in the registry and `docs/calibration.md` (ADR 0005). Known
  simplifications (documented for problem authors):
  - a cache's capacity is per request type;
  - TTL expiry is per key with no stampede coalescing;
  - the return path mirrors the forward hops.
