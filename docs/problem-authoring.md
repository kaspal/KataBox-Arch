# Writing a problem

Problems live in `content/problems/<id>/`. They are packed into the app by `ka-pack` (ADR 0008), simulated by
`ka-sim` (ADR 0006) and judged by `ka-judge` (ADR 0007).

## Start

```sh
cargo run -p ka-pack -- new my-problem   # creates content/problems/my-problem/ from templates
cargo run -p ka-pack -- validate --problems my-problem          # structure, EN/ES parity, designs
cargo run -p ka-pack -- validate --run --problems my-problem    # and judging (slower)
```

Every `TODO(ka-pack)` must go before validation passes.

## The files

| File | What it holds |
|---|---|
| `problem.toml` | Id, number, set, difficulty, titles and "teaches" (EN and ES), tags, NFRs, request types, externals, tests, pass criteria |
| `statement.md`, `statement.es.md` | The brief. Same `##` sections in both languages |
| `hints.md`, `hints.es.md` | Three tiers: `## 1`, `## 2`, `## 3`, from a nudge to almost the answer |
| `editorial.md`, `editorial.es.md` | The worked solution, shown once solved. Same `##` sections in both |
| `reference.kbarch` | A design that is Accepted on every test |
| `naive.kbarch` | The obvious first design: valid, with a path for every request type, but it fails at least one hidden test |

## Request types and routes
A request type has:
- a share of the traffic;
- a route of steps, `role[:qualifier][?]` (ADR 0006, the caller rule):
  - roles: `edge`, `compute`, `worker`, `cache`, `datastore`, `broker` and `external:<name>`;
  - `?` makes a step optional, and `cache?` is cache-aside;
  - `:write` goes to a cluster's primary;
  - `a|b` accepts either;
  - `broker?` followed by its consumer: an optional hand-off. A caller with no broker edge makes the following calls
    itself (inline, so the answer waits for them); use it when the lesson is `queue it or do it now`;
  - `cdc` (right after a `datastore:write` to SQL or NoSQL): the database publishes the committed change to the
    stream on its CDC edge, and the consumer after it becomes the caller. The request's latency ends at the commit,
    so measure the CDC chain with `measure = "completion"` (for example "the search index is fresh within 30 s").
- optionally:
  - `key_space` and `key_skew = { zipf = 1.1 }`;
  - `cacheable` (for CDNs);
  - `payload_kb` and `value_kb`;
  - `api_keys` and `abusive`;
  - `fanout`;
  - `measure = "completion"` (for async work);
  - `source = "scheduler"`.

## Tests
- **A test has:** an id, EN and ES names, `hidden`, `duration_s` (default 60), `traffic` (`flat`, `spike`, `ramp`,
  `steps`, `hourly`), `chaos`, `type_multipliers`, `key_skew_override`, `cold_caches`, and optional `pass`
  overrides.
- **Chaos:**
  - `kill`, `latency`, `partition` (the zone is lost), `split` (the zone keeps running but nothing crosses its
    border), `traffic`, `flush`, `external` (`down`, `add_ms`, or `rate` to cut the provider's rate limit) and
    `model_rate` (a hosted model provider cuts the selected cards' `rpm` / `tpm`, H2);
  - target selectors are written flat: `{ kill = { kind = "sql", role = "primary", at = 30 } }`. A kind selector
    picks the first matching card in the design file (the first placed), so give each data store a different kind
    when a test kills one of them;
  - a zone is a Region or AZ label or id, or `@primary`: the Region holding the main data store's primary (the
    whole design when it isn't in a Region). Use `@primary` in problems: players name their Regions freely.
  - work still waiting when the run and its 30 s drain end counts at its age then, so a stalled pipeline fails its
    completion limit instead of disappearing from it.
  - A worker crash no longer fails its message at once: it comes back after the queue's visibility timeout or the
    stream's session timeout. Make crash tests long enough for that (kill time + timeout + drain).
- **Pass limits:** `p95_ms`, `p99_ms`, `error_rate`, `availability`, `stale_rate`, `lost`, and (H3)
  `dead_lettered` (messages moved to a DLQ: set it wherever a DLQ could hide work that wasn't done). A `p99_ms` limit needs at
  least 1,000 completed requests in the test, or `ka-pack validate --run` refuses the problem.
- **Hidden tests** show their name only, until the problem is solved.
- **Sampling:** tests sample traffic above 5,000 simulated requests per second (ADR 0006). Run the accuracy check
  (`cargo test -p ka-pack --release --test accuracy -- --ignored --nocapture`); if a design's verdict differs between
  sampled and full runs, set a higher `sample_cap` at the top of `problem.toml` and say why in a comment.

## Card properties worth knowing
- **Hosted model limits (H2):** when the statement names the account's limits, pin them in `problem.toml` with
  `[[model_limits]]` (`flavour`, `rpm`, `tpm`): every hosted Model Inference card of that flavour gets them, the
  Inspector shows them read-only, and the card's own values don't count.
- **Rate Limiter `unit`:** `requests` (default) or `tokens` (each request debits its prompt + answer tokens; the
  limit and burst are then tokens per second and tokens), for per-tenant token quotas at an LLM gateway.
- **SQL `read_from`:** `any` (reads spread over the primary and replicas; reads stay in the caller's Region when a
  member is there) or `primary` (read-your-writes).
- **Leases:** a queue's `visibility_timeout_s` and a stream's `session_timeout_s` decide when a dead consumer's
  messages come back.
- **CDC:** a database's `cdc_buffer_s` is how long committed changes wait for a stream that is down.
- **Partitions:** a stream hands out one message per partition at a time, so `partitions` caps how much is consumed at
  once; size it with the consumers.

## Estimation drills
`content/estimation/<problem-id>.toml` holds 2–3 `[[drill]]` entries per problem: `id`, `kind` (qps, storage,
bandwidth, servers, capacity), `prompt`/`prompt_es` with `{input}` placeholders, `inputs`, a `formula` over the inputs
(`+ - * / ^` and parentheses), `unit`, an optional `tolerance` and `solution`/`solution_es` (which may use
`{answer}`). `ka-pack` computes every answer from the formula and refuses unknown or unused inputs. Ground the
inputs in the problem's numbers so the drill teaches something the design needs.

## Find-the-flaw drills
`content/flaws/<id>/` holds `flaw.toml` (story, symptom, explanation, the target test and a `[scenario]` written like a
problem's NFRs, request types and tests), three-tier hints, `broken.kbarch` and `fixed.kbarch`. `validate --run`
proves the drill: the broken design passes Normal (unless the flaw is about Normal) and fails the target test; the
fixed design passes every test.

## Checklist- **Numbers:**
  - NFRs come from the problem's real-world shape;
  - the reference passes with headroom (not by 1%);
  - the naive design fails for the lesson's reason, not by accident.
- **Text:**
  - original, with no copied interview-site material;
  - EN and ES say the same thing.
- **Calibration:** use registry defaults where you can, and change a prop in a design only for a reason the editorial
  explains.
