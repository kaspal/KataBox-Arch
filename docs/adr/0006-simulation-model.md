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
  - a card has `servers = instances × concurrency` (kind defaults when it has no such prop), split into **lanes**
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
  - queues dispatch to any free consumer slot;
  - streams cap in-flight messages at `partitions`;
  - pub/sub pushes one copy per subscriber, and the message completes when all are done.
  - A failed message is retried with exponential backoff up to `max_retries`, then dead-lettered if a DLQ is wired
    (`queue → dlq` Publish), otherwise **lost**.
- **External providers** are scenario entries, not cards. They have a lognormal latency and an optional rate limit;
  chaos can make one fail (1 s connect timeout) or slow.

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
Monthly cost = instances × price + memory × GB price + mean real rps through the card × per-request price. It is
consistent across designs and not a vendor quote.

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
- **Calibration:** the model's knobs are registry props, calibrated in M1.2 with cited sources. Known
  simplifications (documented for problem authors):
  - a cache's capacity is per request type;
  - TTL expiry is per key with no stampede coalescing;
  - the return path mirrors the forward hops.
