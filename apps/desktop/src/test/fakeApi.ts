// An in-memory backend with the same shape as `api`, for UI tests. The registry is the
// fixture Rust exports (`src/test/fixtures/registry.json`), so tests see the real data.
import { vi } from "vitest";
import type { FlawSummary, PackInfo, PackPreview, PathView, AppError, ComposeImport, Design, JsonValue, OpenedDesign, ProblemBrief, RecentEntry, Registry, RunControl, SubmissionSummary, SubmitResult } from "@/bindings";
import { FAKE_BRIEF, FAKE_REFERENCE, fakeJudgement } from "@/test/fakeProblems";
import { fakeFrames } from "@/test/fakeFrames";
import { FAKE_SAMPLE, FAKE_SAMPLE_SUMMARY } from "@/test/fakeSamples";
import { FAKE_DRILLS, FAKE_QUIZ, FAKE_FLOW, FAKE_GUIDE, FAKE_NUMBERS, FAKE_PATTERN, FAKE_TEMPLATE, FAKE_TERMS } from "@/test/fakeGuide";
import type { Api, FramesRequest, RunOptions } from "@/lib/api";
import fixture from "@/test/fixtures/registry.json";

export const REGISTRY = fixture.registry as unknown as Registry;

const UI_SETTINGS = ["theme", "ui_lang", "ui.contrast", "ui.motion", "sandbox.layout", "canvas.snap", "tour.offered", "paths.active", "packs.recommend"];

const err = (code: AppError["code"], message: string = code): AppError => ({ code, message });

export type FakeOptions = {
  settings?: Record<string, JsonValue>;
  /** Quiz questions already due for review (ids). */
  due?: string[];
  /** Files "on disk", by path. */
  files?: Record<string, Design>;
  recent?: RecentEntry[];
  /** What the Open dialog picks (null = cancelled). */
  openPick?: string | null;
  /** What the docker-compose import returns (null = cancelled). */
  composeImport?: ComposeImport | null;
  /** What the Save As dialog picks (null = cancelled). */
  savePick?: string | null;
  devData?: boolean;
  /** Make startRun fail with this error code. */
  runError?: AppError["code"];
  /** What submitting a design to the fake problem returns. */
  verdict?: SubmitResult["judgement"]["verdict"];
  /** What the pack dialog picks: a pack that passes, one that fails its checks, or nothing. */
  pack?: "ok" | "refused" | null;
};

export const FAKE_PACK_PREVIEW: PackPreview = {
  token: 7,
  id: "drills",
  name: "Drills",
  author: "Ana",
  version: "1.0.0",
  licence: "CC-BY-SA-4.0",
  sha256: "c0ffee".padEnd(64, "0"),
  problems: [{ id: "drills--demo", title_en: "Demo drill", title_es: "Ejercicio demo", difficulty: "easy" }],
  change: { kind: "new" },
};

const FAKE_HINTS = { en: ["Look at the reads.", "Cache them.", "Redis in front of the database."], es: ["Mira las lecturas.", "Cachéalas.", "Redis delante de la base de datos."] };

export function createFakeApi(opts: FakeOptions = {}) {
  const settings = new Map(Object.entries(opts.settings ?? {}));
  const files = new Map(Object.entries(opts.files ?? {}).map(([p, d]) => [p, structuredClone(d)]));
  let recent: RecentEntry[] = opts.recent ?? [];
  const known = new Set<string>();
  let dirty = false;
  let title = "";
  let closeListener: (() => void) | null = null;
  let runs = 0;
  let frameSink: ((f: ArrayBuffer) => void) | null = null;
  let runControls: RunControl[] = [];
  let problemState: { status: string | null; best_score: number | null; best_cost: number | null } = { status: null, best_score: null, best_cost: null };
  let hints = 0;
  let draft: Design | null = null;
  let submissionsCount = 0;
  const submitted: SubmissionSummary[] = [];
  const read = new Set<string>();
  const estimates = new Map<string, { tries: number; done: boolean }>();
  const due = new Set<string>(opts.due ?? []);
  let packs: PackInfo[] = [];

  const touch = (path: string, name: string) => {
    recent = [{ path, name, opened_at: 1_700_000_000_000 + recent.length, exists: true }, ...recent.filter((r) => r.path !== path)];
    known.add(path);
  };
  const opened = (path: string): OpenedDesign => {
    const d = files.get(path);
    if (!d) throw err("not_found");
    touch(path, d.name);
    return { path, design: structuredClone(d) };
  };

  const api: Api = {
    appInfo: vi.fn(async () => ({ version: "0.1.0", dev_data: opts.devData ?? true })),
    registry: vi.fn(async () => REGISTRY),
    getSetting: vi.fn(async (key: string) => {
      if (!UI_SETTINGS.includes(key)) throw err("setting_not_allowed");
      return settings.get(key) ?? null;
    }),
    setSetting: vi.fn(async (key: string, value: JsonValue) => {
      if (!UI_SETTINGS.includes(key)) throw err("setting_not_allowed");
      settings.set(key, value);
      return null;
    }),
    recentFiles: vi.fn(async () => recent.map((r) => ({ ...r, exists: files.has(r.path) }))),
    forgetRecent: vi.fn(async (path: string) => {
      recent = recent.filter((r) => r.path !== path);
      return null;
    }),
    openDesignDialog: vi.fn(async () => (opts.openPick ? opened(opts.openPick) : null)),
    importComposeDialog: vi.fn(async () => opts.composeImport ?? null),
    runFrames: vi.fn(async (req: FramesRequest) =>
      fakeFrames(req.source.type === "submission" ? FAKE_REFERENCE : req.source.design, Math.round(req.seconds * req.fps), req.from_s),
    ),
    openRecent: vi.fn(async (path: string) => {
      if (!recent.some((r) => r.path === path)) throw err("path_not_allowed");
      return opened(path);
    }),
    saveDesign: vi.fn(async (path: string, design: Design) => {
      if (!known.has(path)) throw err("path_not_allowed");
      files.set(path, structuredClone(design));
      touch(path, design.name);
      return path;
    }),
    saveDesignAs: vi.fn(async (design: Design) => {
      if (!opts.savePick) return null;
      const path = opts.savePick.endsWith(".kbarch") ? opts.savePick : `${opts.savePick}.kbarch`;
      files.set(path, structuredClone(design));
      touch(path, design.name);
      return path;
    }),
    validateDesign: vi.fn(async () => []),
    startRun: vi.fn(async (design: Design, run: RunOptions, onFrame: (f: ArrayBuffer) => void) => {
      const { chaos, startS } = run;
      if (opts.runError) throw err(opts.runError);
      runs += 1;
      frameSink = onFrame;
      runControls = [];
      return {
        run_id: runs,
        duration_s: 60,
        sample_factor: 1,
        nodes: design.nodes.map((n) => n.id),
        edges: design.edges.map((e) => e.id),
        request_types: ["read", "write"],
        chaos,
        start_s: startS,
      };
    }),
    controlRun: vi.fn(async (_id: number, control: RunControl) => {
      runControls.push(control);
      return null;
    }),
    runResult: vi.fn(async () => null),
    listProblems: vi.fn(async () => [
      { ...FAKE_BRIEF.summary, ...problemState },
      ...packs
        .filter((p) => !p.removed)
        .map((p) => ({ ...FAKE_BRIEF.summary, id: `${p.id}--demo`, number: 1, title_en: "Demo drill", title_es: "Ejercicio demo", status: null, best_score: null, best_cost: null, pack: p.name })),
    ]),
    problemBrief: vi.fn(async (id: string): Promise<ProblemBrief> => {
      if (id !== FAKE_BRIEF.summary.id) throw err("not_found");
      return {
        ...FAKE_BRIEF,
        summary: { ...FAKE_BRIEF.summary, ...problemState },
        hints_unlocked: hints,
        hints_en: FAKE_HINTS.en.slice(0, hints),
        hints_es: FAKE_HINTS.es.slice(0, hints),
        solved: problemState.status === "solved",
        editorial_en: problemState.status === "solved" ? "## Editorial" : null,
        editorial_es: problemState.status === "solved" ? "## Editorial" : null,
        reference: problemState.status === "solved" ? FAKE_REFERENCE : null,
        reference_point: problemState.status === "solved" ? { cost_usd: 1600, p95_ms: 24.5 } : null,
        draft,
      };
    }),
    unlockHint: vi.fn(async (_id: string, tier: number) => {
      hints = Math.max(hints, tier);
      return null;
    }),
    saveDraft: vi.fn(async (_id: string, d: Design) => {
      draft = structuredClone(d);
      return null;
    }),
    submitDesign: vi.fn(async (id: string): Promise<SubmitResult> => {
      const judgement = fakeJudgement(id, opts.verdict ?? "accepted");
      const solved = judgement.verdict === "accepted";
      const first = solved && problemState.status !== "solved";
      problemState = { status: solved || problemState.status === "solved" ? "solved" : "attempted", best_score: Math.max(problemState.best_score ?? 0, judgement.score.total), best_cost: null };
      submitted.unshift({
        id: ++submissionsCount,
        problem_id: id,
        created_at: submissionsCount,
        verdict: judgement.verdict,
        score: judgement.score.total,
        correctness: judgement.score.correctness,
        cost_score: judgement.score.cost,
        latency_score: judgement.score.latency,
        cost_usd: judgement.cost_month,
        p95_ms: judgement.tests[0].p95_ms,
      });
      return { submission_id: submissionsCount, judgement, first_solve: first, best_score: problemState.best_score ?? 0 };
    }),
    submissions: vi.fn(async () => [...submitted]),
    listGuide: vi.fn(async () =>
      FAKE_GUIDE.map((e) => ({ kind: e.kind, one_liner: e.one_liner, one_liner_es: e.one_liner_es, read: read.has(e.kind) })),
    ),
    guidePage: vi.fn(async (kind: string) => {
      const entry = FAKE_GUIDE.find((e) => e.kind === kind);
      if (!entry) throw err("not_found");
      return { entry: structuredClone(entry), md_en: "A cache keeps **hot** data in memory.", md_es: "Una caché guarda datos **calientes** en memoria.", read: read.has(kind) };
    }),
    decideFlows: vi.fn(async () => [structuredClone(FAKE_FLOW)]),
    glossary: vi.fn(async () => structuredClone(FAKE_TERMS)),
    numbers: vi.fn(async () => structuredClone(FAKE_NUMBERS)),
    drills: vi.fn(async (problemId?: string) => structuredClone(FAKE_DRILLS.filter((d) => !problemId || d.problem_id === problemId))),
    checkEstimate: vi.fn(async (problemId: string, drillId: string, given: number) => {
      const d = FAKE_DRILLS.find((x) => x.problem_id === problemId && x.id === drillId);
      if (!d) throw err("not_found");
      // The fake's answer is the first input over 86,400 (its drills are QPS drills).
      const answer = Object.values(d.inputs)[0]! / 86_400;
      const ratio = given / answer;
      const ok = ratio <= 1.5 && ratio >= 1 / 1.5;
      const closeness = ok ? "close" : ratio > 10 || ratio < 0.1 ? "way_off" : ratio > 1 ? "too_high" : "too_low";
      const key = `${problemId}/${drillId}`;
      const s = estimates.get(key) ?? { tries: 0, done: false };
      estimates.set(key, { tries: s.tries + 1, done: s.done || ok });
      return { ok, closeness, answer, solution: "{daily} ÷ 86,400 = {answer}", solution_es: "{daily} ÷ 86 400 = {answer}", inputs: d.inputs } as const;
    }),
    learningPaths: vi.fn(async (): Promise<PathView[]> => [
      {
        id: "foundations",
        number: 1,
        title: "Foundations",
        title_es: "Fundamentos",
        blurb: "The pieces almost every design uses.",
        blurb_es: "Las piezas que usa casi cualquier diseño.",
        items: [
          { kind: "guide", target: "cache", title: null, title_es: null, done: read.has("cache") },
          { kind: "problem", target: "url-shortener", title: "URL Shortener", title_es: "Acortador de URLs", done: false },
          { kind: "flaw", target: "cache-no-ttl", title: "A cache that never forgets", title_es: "Una caché que nunca olvida", done: false },
        ],
      },
      { id: "async-systems", number: 2, title: "Async systems", title_es: "Sistemas asíncronos", blurb: "Queues.", blurb_es: "Colas.", items: [{ kind: "guide", target: "queue", title: null, title_es: null, done: false }] },
    ]),
    setConceptPlayed: vi.fn(async () => {}),
    quiz: vi.fn(async (page: string) =>
      FAKE_QUIZ.map((q, i) => ({ id: `${page}:${i + 1}`, q: q.q, q_es: q.q_es, options: q.options, options_es: q.options_es })),
    ),
    answerQuiz: vi.fn(async (questionId: string, choice: number) => {
      const q = FAKE_QUIZ[Number(questionId.split(":").pop()) - 1];
      if (!q) throw err("not_found");
      const correct = choice === q.answer;
      if (!correct) due.add(questionId);
      return { correct, answer: q.answer, why: q.why, why_es: q.why_es, due_on: correct ? null : "2026-11-23" };
    }),
    reviewsDue: vi.fn(async () =>
      [...due].map((id) => {
        const q = FAKE_QUIZ[Number(id.split(":").pop()) - 1]!;
        return { id, q: q.q, q_es: q.q_es, options: q.options, options_es: q.options_es };
      }),
    ),
    listFlaws: vi.fn(async (): Promise<FlawSummary[]> => [
      { id: "cache-no-ttl", number: 1, title: "A cache that never forgets", title_es: "Una caché que nunca olvida", kinds: ["cache"], solved: false },
      { id: "single-primary", number: 2, title: "One database, no plan B", title_es: "Una base de datos, sin plan B", kinds: ["sql"], solved: true },
    ]),
    estimateStatuses: vi.fn(async () =>
      [...estimates].map(([key, s]) => {
        const [problem_id, drill_id] = key.split("/");
        return { problem_id: problem_id!, drill_id: drill_id!, tries: s.tries, done: s.done };
      }),
    ),
    listPatterns: vi.fn(async () => [
      { id: FAKE_PATTERN.id, number: 1, title: FAKE_PATTERN.title, title_es: FAKE_PATTERN.title_es, one_liner: FAKE_PATTERN.one_liner, one_liner_es: FAKE_PATTERN.one_liner_es, kinds: FAKE_PATTERN.kinds, template: structuredClone(FAKE_TEMPLATE), read: read.has(`pattern:${FAKE_PATTERN.id}`) },
    ]),
    pattern: vi.fn(async (id: string) => {
      if (id !== FAKE_PATTERN.id) throw err("not_found");
      return { entry: structuredClone(FAKE_PATTERN), md_en: "Read the **cache** first.", md_es: "Lee primero la **caché**.", template: structuredClone(FAKE_TEMPLATE), read: read.has(`pattern:${id}`) };
    }),
    setPatternRead: vi.fn(async (id: string, r: boolean) => {
      if (r) read.add(`pattern:${id}`);
      else read.delete(`pattern:${id}`);
      return null;
    }),
    concept: vi.fn(async (kind: string) =>
      kind === "cache" ? { ...structuredClone(FAKE_SAMPLE), def: { ...structuredClone(FAKE_SAMPLE.def), id: "cache", problem: null, steps: FAKE_SAMPLE.def.steps.slice(0, 1) } } : null,
    ),
    setGuideRead: vi.fn(async (kind: string, r: boolean) => {
      if (r) read.add(kind);
      else read.delete(kind);
      return null;
    }),
    listSamples: vi.fn(async () => [structuredClone(FAKE_SAMPLE_SUMMARY)]),
    sample: vi.fn(async (id: string) => {
      if (id !== FAKE_SAMPLE.def.id) throw err("not_found");
      return structuredClone(FAKE_SAMPLE);
    }),
    progress: vi.fn(async () => ({
      easy: { total: 1, attempted: problemState.status === "attempted" ? 1 : 0, solved: problemState.status === "solved" ? 1 : 0 },
      medium: { total: 0, attempted: 0, solved: 0 },
      hard: { total: 0, attempted: 0, solved: 0 },
      streak: problemState.status ? 1 : 0,
      recommended: problemState.status === "solved" ? null : FAKE_BRIEF.summary.id,
      continue_with: problemState.status === "attempted" ? FAKE_BRIEF.summary.id : null,
    })),
    findBreakingPoint: vi.fn(async (design: Design) => {
      if (opts.runError) throw err(opts.runError);
      return {
        limit_rps: 31_250,
        multiplier: 31.25,
        base_rps: 1000,
        beyond_ceiling: false,
        first_failure: { node: design.nodes[design.nodes.length - 1]?.id ?? "x", reason: "saturated" as const, at_s: 4, utilisation: 0.97 },
        failing_p95_ms: 480,
        failing_error_rate: 0.02,
        stale_rate: 0.012,
        runs: 14,
      };
    }),
    setWindowTitle: vi.fn(async (t: string) => {
      title = t;
      return null;
    }),
    setDirty: vi.fn(async (d: boolean) => {
      dirty = d;
      return null;
    }),
    confirmClose: vi.fn(async () => null),
    kataboxLinks: vi.fn(async (target: string) =>
      target === "guide:cache" || target === "problem:url-shortener"
        ? [{ kind: "learn" as const, id: "linear/lru-cache", title: "LRU cache", title_es: "Caché LRU" }, { kind: "problem" as const, id: "league-rank", title: "League Rank", title_es: "Clasificación de la liga" }]
        : [],
    ),
    saveExport: vi.fn(async (_f: string, _d: Uint8Array, name: string) => `C:/exports/${name}`),
    mermaidText: vi.fn(async () => "flowchart LR\n"),
    saveMermaid: vi.fn(async (_d: Design, _n: Record<string, string>, name: string) => `C:/exports/${name}`),
    previewPack: vi.fn(async () => {
      if (opts.pack === "refused") throw err("pack_refused", "problems/demo/statement.md: a <script> tag is not allowed in pack text");
      return opts.pack === "ok" ? structuredClone(FAKE_PACK_PREVIEW) : null;
    }),
    applyPack: vi.fn(async (): Promise<PackInfo> => {
      const p = FAKE_PACK_PREVIEW;
      const info = { id: p.id, name: p.name, author: p.author, version: p.version, licence: p.licence, sha256: p.sha256, installed_at: 1, removed: false, problems: 1 };
      packs = [...packs.filter((x) => x.id !== p.id), info];
      return info;
    }),
    cancelPack: vi.fn(async () => null),
    listPacks: vi.fn(async () => structuredClone(packs)),
    uninstallPack: vi.fn(async (id: string) => {
      packs = packs.map((p) => (p.id === id ? { ...p, removed: true, problems: 0 } : p));
      return null;
    }),
    onCloseRequested: vi.fn(async (cb: () => void) => {
      closeListener = cb;
      return () => {
        closeListener = null;
      };
    }),
  };

  return {
    api,
    files,
    settings,
    get title() {
      return title;
    },
    get dirty() {
      return dirty;
    },
    /** Push a frame to the live run's channel, as Rust would. */
    pushFrame: (frame: ArrayBuffer) => frameSink?.(frame),
    get runs() {
      return runs;
    },
    get runControls() {
      return runControls;
    },
    /** Simulate the user closing the window (Rust held it back). */
    requestClose: () => closeListener?.(),
  };
}
