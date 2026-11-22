// An in-memory backend with the same shape as `api`, for UI tests. The registry is the
// fixture Rust exports (`src/test/fixtures/registry.json`), so tests see the real data.
import { vi } from "vitest";
import type { AppError, Design, JsonValue, OpenedDesign, ProblemBrief, RecentEntry, Registry, RunControl, SubmissionSummary, SubmitResult } from "@/bindings";
import { FAKE_BRIEF, FAKE_REFERENCE, fakeJudgement } from "@/test/fakeProblems";
import type { Api, RunOptions } from "@/lib/api";
import fixture from "@/test/fixtures/registry.json";

export const REGISTRY = fixture.registry as unknown as Registry;

const UI_SETTINGS = ["theme", "ui_lang", "ui.contrast", "ui.motion", "sandbox.layout", "canvas.snap"];

const err = (code: AppError["code"], message = code): AppError => ({ code, message });

export type FakeOptions = {
  settings?: Record<string, JsonValue>;
  /** Files "on disk", by path. */
  files?: Record<string, Design>;
  recent?: RecentEntry[];
  /** What the Open dialog picks (null = cancelled). */
  openPick?: string | null;
  /** What the Save As dialog picks (null = cancelled). */
  savePick?: string | null;
  devData?: boolean;
  /** Make startRun fail with this error code. */
  runError?: AppError["code"];
  /** What submitting a design to the fake problem returns. */
  verdict?: SubmitResult["judgement"]["verdict"];
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
    listProblems: vi.fn(async () => [{ ...FAKE_BRIEF.summary, ...problemState }]),
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
