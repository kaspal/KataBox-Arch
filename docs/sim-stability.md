# Simulation stability

This file records the gate checks on the simulator.
- **Accuracy (review A1, ADR 0006):** sampled runs against full runs at peak load.
- **Determinism:** 100 runs of one seed must be byte-identical.
- **Spread:** p95 and utilisation across 100 seeds.

Problems' scores join these checks at M2.8.

Run them in release:

```sh
cargo test -p ka-sim --release --test sim --test stability -- --ignored --nocapture
```

## Sampled vs full at peak load

| Case | f | p95 full | p95 sampled | Δ | Utilisation (largest gap) | Verdict |
|---|---|---|---|---|---|---|
| golden a (1k rps) | 1.000 | 27.126 ms | 27.126 ms | 0.0% | 0 (not sampled) | same |
| golden b (12k rps) | 0.417 | 21.817 ms | 21.817 ms | 0.0% (same 2% bucket) | db-replica 0.0448 vs 0.0456 (+1.8%) | same |
| golden b (50k rps, saturated) | 0.100 | 85.546 ms | 87.257 ms | +2.0% | db-replica 0.1128 vs 0.1202 (+6.6%) | same (fail) |
| golden c (400 msg/s) | 1.000 | 68.801 ms | 68.801 ms | 0.0% | 0 (not sampled) | same |

- **M1.8 gate (seed 11):** every case is within ±10%, with the same verdict, so no `sample_cap` raise was needed.
- **Earlier (M1.1):** golden b at 50k was +6.1% p95 and −6% cache utilisation. The M1.2 calibration (Redis at 0.01
  ms) took the cache out of saturation.

## Determinism (M1.8)

100 runs of seed 42 give byte-identical `RunResult` JSON for every golden: a (5,504 bytes), b at 12k (10,289 bytes)
and c (7,492 bytes).

Live runs add two more checks:
- tracing and frame-sized steps leave results identical (`ka-sim` tests);
- a seek lands in the same state as playing through (`ka-app` tests).

## Spread across 100 seeds (M1.8)

| Golden | p95 mean (ms) | p95 min–max | p95 CV | busiest util mean | util min–max |
|---|---|---|---|---|---|
| a | 26.95 | 26.59–27.13 | 0.9% | 0.107 | 0.105–0.108 |
| b (12k) | 21.82 | 21.82–21.82 | 0.0% | 0.264 | 0.262–0.266 |
| c | 69.56 | 67.45–71.58 | 1.2% | 0.375 | 0.366–0.384 |

- **Golden b's p95** falls in the same 2% histogram bucket for every seed.
- **The test fails** if any golden's p95 varies by more than 10% across seeds: a verdict mustn't hinge on the seed.

## Engine bench

| Gate | 60 s of golden b at 50k rps (release) |
|---|---|
| M1.1 | 0.44 s |
| M1.8 | 0.47 s |

Both runs had a sampling factor of 0.10 and about 3.0M weighted requests.
