// An in-memory backend with the same shape as `api`, for UI tests. The registry is the
// fixture Rust exports (`src/test/fixtures/registry.json`), so tests see the real data.
import { vi } from "vitest";
import type { AppError, ChaosEvent, Design, JsonValue, OpenedDesign, RecentEntry, Registry, RunControl, SandboxPreset } from "@/bindings";
import type { Api } from "@/lib/api";
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
};

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
    startRun: vi.fn(async (design: Design, _preset: SandboxPreset, _speed: number, chaos: ChaosEvent[], startS: number, onFrame: (f: ArrayBuffer) => void) => {
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
