// A one-problem catalogue for UI tests (the shapes Rust sends: ka-app problems.rs).
import type { Judgement, ProblemBrief, Verdict } from "@/bindings";

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
  },
  nfr: { peak_rps: 12000, read_ratio: 0.95, p95_ms: 80, availability: 0.999, budget_usd: 2500 },
  statement_en: "# URL Shortener\n\n## The problem\n\nTurn long links into short ones. <b>raw</b>\n\n| a | b |\n|---|---|\n| 1 | 2 |",
  statement_es: "# Acortador de URLs\n\n## El problema\n\nConvierte enlaces largos en cortos.",
  request_types: [
    { id: "redirect", name: "Redirect", name_es: "Redirección", share: 0.95 },
    { id: "create", name: "Create a link", name_es: "Crear un enlace", share: 0.05 },
  ],
  externals: [],
  tests: [
    { id: "normal", name: "Normal day", name_es: "Un día normal", hidden: false, duration_s: 60 },
    { id: "viral", name: "Link goes viral", name_es: "El enlace se hace viral", hidden: true, duration_s: 60 },
  ],
  hints_unlocked: 0,
  hints_en: [],
  hints_es: [],
  solved: false,
  editorial_en: null,
  editorial_es: null,
  reference: null,
  draft: null,
  runs: 0,
};

export function fakeJudgement(problemId: string, verdict: Verdict): Judgement {
  const pass = verdict === "accepted";
  const limits = { p95_ms: 80, error_rate: 0.001, availability: 0.999, stale_rate: null, lost: null };
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
        error_rate: 0,
        availability: 1,
        stale_rate: 0,
        lost: 0,
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
        error_rate: pass ? 0 : 0.034,
        availability: pass ? 1 : 0.966,
        stale_rate: 0,
        lost: 0,
        throughput_rps: 30500,
        cost_month: 2100,
        limits,
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
