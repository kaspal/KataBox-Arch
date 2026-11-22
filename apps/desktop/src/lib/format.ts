// Shared number formats.
import type { TFunction } from "i18next";
import type { LatencyMeasure } from "@/bindings";

/** A latency: milliseconds below a second ("31.2 ms"), seconds above ("25.1 s"). */
export function formatMs(lang: string, ms: number): string {
  const n = new Intl.NumberFormat(lang, { maximumFractionDigits: 1 });
  return ms < 1000 ? `${n.format(ms)} ms` : `${n.format(ms / 1000)} s`;
}

/** The label of a percentile, by what the problem's latency limits measure (review F2). */
export function percentileLabel(t: TFunction, p: "p95" | "p99", judged: LatencyMeasure): string {
  return t(`latency.${p}.${judged}` as "latency.p95.response");
}
