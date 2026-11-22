# Simulation stability

Records the sampled-vs-full accuracy check (review A1, ADR 0006) and, from M1.8, the score distributions across
seeds. Run the accuracy check with `cargo test -p ka-sim --release -- --ignored --nocapture accuracy`.

## Sampled vs full at peak load (M1.1, seed 11)

| Case | f | p95 full | p95 sampled | Δ | Utilisation (largest gap) | Verdict |
|---|---|---|---|---|---|---|
| golden a (1k rps) | 1.000 | 27.126 ms | 27.126 ms | 0.0% | 0 (not sampled) | same |
| golden b (12k rps) | 0.417 | 21.817 ms | 21.817 ms | 0.0% (same 2% bucket) | db-replica 0.0448 vs 0.0456 | same |
| golden b (50k rps, saturated) | 0.100 | 87.257 ms | 92.597 ms | +6.1% | cache 0.963 vs 0.906 (−6%) | same (fail) |
| golden c (400 msg/s) | 1.000 | 68.801 ms | 68.801 ms | 0.0% | 0 (not sampled) | same |

All within ±10%; no `sample_cap` raise needed.

## Engine bench

60 s of golden b at 50k rps: **0.44 s** in release (sampling factor 0.10, about 3.0M weighted requests).
