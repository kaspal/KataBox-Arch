# ADR 0005: Calibration of simulator figures

- Status: accepted (M1.2, 2026-11-22)

## Context

The simulator's verdicts depend on figures such as service times, concurrency, cold starts, replication lag, queue
sizes and prices. A learner should be able to ask "why 5 ms?" and get an answer. A problem author should be able to
trust that the same card behaves the same in every problem.

Until M1.2 these figures were spread around: some were registry prop defaults, others were hard-coded per kind in
`ka-sim`.

## Decision

### One home for every figure
- **The registry holds them all**, as data (`crates/ka-core/src/registry.rs`):
  - **Calibrated props.** Defaults of the props in `CALIBRATED_PROPS`: `service_ms`, `concurrency`,
    `concurrency_limit`, `cold_start_ms`, `replication_lag_ms`, `failover_s` and `visibility_timeout_s`. Flavours may
    override them (MySQL's 151 connections, Sidekiq's 5 threads).
  - **Behaviour without a prop.** `FlavourSpec.sim: SimDefaults`, made of `servers_per_instance`, `service_ms`,
    `p99_ratio`, `queue_per_server` and `failover_s`. Kinds never on a request's path have none. `ka-sim` reads these
    and no longer keeps its own table.
  - **Prices.** `FlavourSpec.cost` (ADR 0006).
- **Design choices are not calibrated.** Props such as instances, shards, partitions, memory, TTLs, limits and
  retries have defaults that are only starting points.

### One row per figure
- **The doc:** `docs/calibration.md` has one row per figure: id, value, unit and source.
- **The test:** `crates/ka-core/tests/calibration.rs` reads it and fails if any of these hold:
  - a figure has no row;
  - a value differs from the registry;
  - a row is stale;
  - a status doesn't match its source.

  The doc can't drift from the code.
- **Row ids** come from `calibration_rows()`:
  - `<kind>.<prop>` for a calibrated prop, with `@<flavour>` for an override;
  - `<kind>.<field>` for behaviour without a prop;
  - `<kind>.cost.instance|gb|million` for prices.

### Sources and honesty
- **Two statuses.** Each calibrated prop carries `calibration: { id, status }`:
  - `cited` when the source has been checked;
  - `needed` when the figure is a conservative ballpark.
- **The doc marks every ballpark.** Each starts `source: needed (suggested: <vendor page>)`.
- **Nothing uncited is presented as cited.** The M1–M3 run was unattended and couldn't open web pages, so every
  figure starts as `needed`.
- **Confirming a figure:**
  1. replace its source with the citation;
  2. add its id to `CITED`;
  3. the inspector's tag changes from "ballpark" to "cited".
- **The inspector:**
  - tags each calibrated prop "ballpark" or "cited", with an explanation on hover;
  - explains that untagged props are design choices;
  - no longer shows the M0 "defaults are placeholders" note.

### Changing a figure
- **Change it in the registry and the doc together.** The test enforces this.
- **Goldens** (`crates/ka-sim/tests/golden`) are regenerated on purpose, with the reason in the commit message.
- **From M2:** problem references are re-validated with `ka-pack validate --run`.

## Consequences
- **One place to answer "why".** Every number the simulator uses can be traced to one row. The Component Guide (M3)
  quotes those values from the registry, never from copied text.
- **The ballparks still need checking.** The sources still to confirm are listed in the M1–M3 report for the
  maintainer to check.
- **The first calibration changed some behaviour:**
  - a Redis cache now serves about 100k ops/s;
  - DynamoDB is about 2.7k ops/s per partition;
  - search runs 13 threads per shard;
  - brokers add their publish latency to the publish acknowledgement.
