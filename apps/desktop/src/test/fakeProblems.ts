// A one-problem catalogue for UI tests (the shapes Rust sends: ka-app problems.rs).
import type { Design, Judgement, ProblemBrief, Verdict } from "@/bindings";

export const FAKE_BRIEF: ProblemBrief = {
  summary: {
    id: "url-shortener",
    number: 1,
    set: 1,
    difficulty: "easy",
    title_en: "URL Shortener",
    title_es: "Acortador de URLs",
    teaches_en: "Caching, replicas, failover",
    teaches_es: "Caché, réplicas, failover",
    tags: ["caching", "read-heavy"],
    status: null,
    best_score: null,
    best_cost: null,
    pack: null,
  },
  nfr: { peak_rps: 12000, read_ratio: 0.95, p95_ms: 80, availability: 0.999, budget_usd: 2500 },
  p99_ms: 120,
  judged_on: "response",
  statement_en: "# URL Shortener\n\n## The problem\n\nTurn long links into short ones. <b>raw</b>\n\n| a | b |\n|---|---|\n| 1 | 2 |",
  statement_es: "# Acortador de URLs\n\n## El problema\n\nConvierte enlaces largos en cortos.",
  request_types: [
    { id: "redirect", name: "Redirect", name_es: "Redirección", share: 0.95 },
    { id: "create", name: "Create a link", name_es: "Crear un enlace", share: 0.05 },
  ],
  externals: [],
  tests: [
    { id: "normal", name: "Normal day", name_es: "Un día normal", hidden: false, duration_s: 60, availability: 0.99 },
    { id: "viral", name: "Link goes viral", name_es: "El enlace se hace viral", hidden: true, duration_s: 60, availability: null },
  ],
  hints_unlocked: 0,
  hints_en: [],
  hints_es: [],
  solved: false,
  editorial_en: null,
  editorial_es: null,
  reference: null,
  reference_point: null,
  draft: null,
  runs: 0,
};

/** The reference design the brief carries once solved. */
export const FAKE_REFERENCE: Design = {
  schema_version: 2,
  id: "url-shortener-reference",
  name: "URL Shortener (reference)",
  problem_id: "url-shortener",
  nodes: [
    { id: "ref-app", kind: "app", flavour: "go", label: "", position: { x: 0, y: 0 }, props: {}, group_id: null },
    { id: "ref-cache", kind: "cache", flavour: "redis", label: "", position: { x: 0, y: 160 }, props: {}, group_id: null },
  ],
  edges: [{ id: "ref-e1", from: "ref-app", to: "ref-cache", protocol: "query" }],
  groups: [],
  notes: [],
  viewport: { x: 0, y: 0, zoom: 1 },
};

export function fakeJudgement(problemId: string, verdict: Verdict): Judgement {
  const pass = verdict === "accepted";
  const limits = { p95_ms: 80, p99_ms: null, error_rate: 0.001, availability: 0.999, stale_rate: null, lost: null };
  return {
    problem_id: problemId,
    verdict,
    lint: [],
    tests: [
      {
        id: "normal",
        name: "Normal day",
        name_es: "Un día normal",
        hidden: false,
        verdict: "accepted",
        p95_ms: 31.2,
        p99_ms: 48.5,
        error_rate: 0,
        availability: 1,
        stale_rate: 0,
        lost: 0,
        retried: 0,
        throughput_rps: 11980,
        cost_month: 1850,
        limits,
        bottleneck: null,
        replay_at_s: null,
        seed: 1,
      },
      {
        id: "viral",
        name: "Link goes viral",
        name_es: "El enlace se hace viral",
        hidden: true,
        verdict: pass ? "accepted" : verdict,
        p95_ms: pass ? 54 : 412,
        p99_ms: pass ? 80 : 900,
        error_rate: pass ? 0 : 0.034,
        availability: pass ? 1 : 0.966,
        stale_rate: 0,
        lost: pass ? 0 : 12,
        retried: pass ? 0 : 1234,
        throughput_rps: 30500,
        cost_month: 2100,
        limits: { ...limits, p99_ms: 120 },
        bottleneck: pass ? null : { node: "db", kind: "sql", from_s: 22, utilisation: 0.99, reason: "saturated" },
        replay_at_s: pass ? null : 20,
        seed: 2,
      },
    ],
    score: pass ? { correctness: 70, cost: 17, latency: 8, total: 95 } : { correctness: 35, cost: 9, latency: 6, total: 50 },
    cost_month: 1850,
    ref_cost: 1600,
  };
}
