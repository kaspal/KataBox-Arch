# ADR 0007: The judge and the score

- Status: accepted (M2.2, 2026-11-22)

## Context

Submitting a design to a problem must give the same verdict and score every time, on every machine, and must say
why it failed in terms a learner can act on. Hidden tests must stay hidden. The AI review (M4) explains results; it
never changes them.

## Decision

### `ka-judge`
- **`judge(problem, design, registry, ref_cost) -> Judgement`.** The crate is a pure library, with the same
  determinism lints as `ka-sim` (`clippy.toml`).
- **Lint first:**
  - design validation;
  - a path for every request type (compiling the first test's scenario);
  - the kinds the problem `requires`.

  Any finding makes the whole judgement **Invalid Design**, with the reasons listed and a score of 0.
- **Each test** runs in a fresh engine, with its own seed `fnv64("<problem>/<test>")`.
- **Per-test verdicts,** in this order (the first that applies wins):
  1. **Invalid Design**;
  2. **Dropped Requests:** the error rate is over `pass.error_rate`, or lost messages are over `pass.lost`
     (default 0);
  3. **SLO Breach:** p95 is over the target, availability is under the NFR, or the stale-read rate is over
     `pass.stale_rate`;
  4. **Over Budget:** the monthly cost on the first test is over `budget_usd`;
  5. **Accepted**.
- **Limits.** A test is held to the problem's `pass` criteria, then the NFR's p95 and availability where `pass`
  leaves them unset, then the test's own overrides.
- **The judgement's verdict** is Accepted if every test is; otherwise it is the most severe test verdict.

### Score (out of 100; review A3)
| Part | Formula |
|---|---|
| Correctness | 70 × accepted / total |
| Cost | 20 × clamp(ref_cost / cost, 0, 1) × accepted / total |
| Latency | 10 × min(1, mean over accepted tests of clamp(1 − p95 / target, 0, 1) / 0.5); 0 if none passed |

- **Rounding.** Each part is rounded half to even, and the total is their sum. So 3 of 4 tests gives 52 / 70, as in
  the mockup.
- **Costs.** `cost` is the design's monthly cost on the first test. `ref_cost` is the reference's, stored in
  `content.db` by `ka-pack`.
- **No reward for failing cheaply.** A design that passes nothing earns no cost or latency points.

### Bottleneck and replay
- **The bottleneck** of a failing test is the earliest second in which a card is **down**, or at least 95% busy with
  its **queue overflowing** (drops) or **growing** (saturated). Ties go to the busiest, then the lowest id.
- **Replay at failure** starts the test's live run 2 s before that second, or at 0.

### Hidden tests
- **Results only.** Test results carry the name (EN and ES), the verdict, the metrics, the limits, the bottleneck
  and the seed, but never the traffic, chaos or multipliers. A test checks that the serialized judgement has none
  of them.
- **Live replays.** The app loads a hidden test's definition in Rust when it starts a replay; it never sends it.

### Content checks
`ka-pack validate --run` judges every problem's reference (it must be Accepted on every test, against its own cost)
and naive design (it must be valid and fail at least one hidden test).

## Consequences
- **Reproducible.** Scores can be stored and compared across versions as long as the simulator's goldens don't
  change. A golden change is a reviewed, intentional event (ADR 0006).
- **Explainable.** The bottleneck and the replay point give the Results page and the AI review a concrete place to
  start.
