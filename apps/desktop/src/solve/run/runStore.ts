// The live run's state, outside the editor's undo history: status, the latest frame, the
// per-second series, injected chaos and the controls. Rust owns the run
// (ka-app/src/live.rs); this store only mirrors it.
import { create } from "zustand";
import type { ChaosEvent, Design, RunInfo, SandboxPreset } from "@/bindings";
import { api } from "@/lib/api";
import { decodeFrame, type Frame } from "@/solve/run/frames";

export type RunStatus = "idle" | "starting" | "playing" | "paused" | "finished";

export const SPEEDS = [0.25, 0.5, 1, 2, 4, 8] as const;

/** One completed simulated second, as frames report it. */
export type SeriesPoint = { t: number; p95Ms: number; rps: number; errorRate: number };

type RunState = {
  status: RunStatus;
  info: RunInfo | null;
  /** The latest frame; kept while an edited design restarts, until the new run's first frame. */
  frame: Frame | null;
  /** Completed seconds so far (for the sparkline), in time order. */
  series: SeriesPoint[];
  preset: SandboxPreset;
  speed: number;
  /** Chaos the user injected into this run, each at the moment it was injected. */
  injected: ChaosEvent[];
  /** What the running design looked like to the simulator (see `simKey`). */
  startedKey: string | null;
  start: (design: Design, opts?: { startS?: number; keepInjected?: boolean }) => Promise<void>;
  /** Add `event` at the current time and continue from there (a fresh, deterministic run). */
  inject: (design: Design, event: ChaosEvent) => Promise<void>;
  pause: () => void;
  resume: () => void;
  seek: (seconds: number) => void;
  setSpeed: (speed: number) => void;
  setPreset: (preset: SandboxPreset) => void;
  stop: () => void;
};

/** The parts of a design the simulator reads: moving a card doesn't restart a run. */
export function simKey(d: Design): string {
  return JSON.stringify([
    d.id,
    d.nodes.map((n) => [n.id, n.kind, n.flavour, n.props, n.group_id]),
    d.edges,
    d.groups.map((g) => [g.id, g.kind, g.parent_id, g.scaling ?? null]),
  ]);
}

/** A chaos event with its time moved to `at` seconds. */
export function chaosAt(e: ChaosEvent, at: number): ChaosEvent {
  const [k, v] = Object.entries(e)[0] as [string, Record<string, unknown>];
  return { [k]: { ...v, at } } as ChaosEvent;
}

/** When a chaos event happens, in seconds. */
export function chaosTime(e: ChaosEvent): number {
  return (Object.values(e)[0] as { at: number }).at;
}

// Each start (or stop) bumps the token; frames and answers of older runs are ignored.
let token = 0;
const frameListeners = new Set<(f: Frame) => void>();

/** Called on every frame (the packet layer). Returns an unsubscribe function. */
export function onFrame(cb: (f: Frame) => void): () => void {
  frameListeners.add(cb);
  return () => frameListeners.delete(cb);
}

/** Hand a frame to the listeners without a run behind it (the dev perf page). */
export function emitFrame(f: Frame) {
  frameListeners.forEach((l) => l(f));
}

/** Fold a frame's "last completed second" into the series (seeks truncate it). */
export function addToSeries(series: SeriesPoint[], f: Frame): SeriesPoint[] {
  const t = Math.floor(f.tMs / 1000) - 1;
  if (t < 0) return series.length ? [] : series;
  const last = series[series.length - 1];
  if (last && last.t === t) return series;
  const point = { t, p95Ms: f.p95Ms, rps: f.rps, errorRate: f.errorRate };
  if (last && last.t > t) return [...series.filter((p) => p.t < t), point];
  return [...series, point];
}

export const useRun = create<RunState>()((set, get) => {
  const send = (control: Parameters<typeof api.controlRun>[1]) => {
    const info = get().info;
    if (info) void api.controlRun(info.run_id, control).catch(() => {});
  };
  return {
    status: "idle",
    info: null,
    frame: null,
    series: [],
    preset: "normal",
    speed: 1,
    injected: [],
    startedKey: null,

    start: async (design, opts = {}) => {
      const mine = ++token;
      const injected = opts.keepInjected ? get().injected : [];
      set({ status: "starting", startedKey: simKey(design), injected });
      try {
        const info = await api.startRun(design, get().preset, get().speed, injected, opts.startS ?? 0, (buf) => {
          if (mine !== token) return;
          let f: Frame;
          try {
            f = decodeFrame(buf);
          } catch {
            return;
          }
          set((s) => ({
            frame: f,
            series: addToSeries(s.series, f),
            status: f.finished ? "finished" : f.playing ? "playing" : "paused",
          }));
          frameListeners.forEach((l) => l(f));
        });
        if (mine === token) set({ info });
      } catch (e) {
        if (mine === token) set({ status: "idle", info: null, startedKey: null });
        throw e;
      }
    },
    inject: async (design, event) => {
      const at = (get().frame?.tMs ?? 0) / 1000;
      set((s) => ({ injected: [...s.injected, chaosAt(event, at)] }));
      await get().start(design, { startS: at, keepInjected: true });
    },
    pause: () => {
      send("pause");
      if (get().status === "playing") set({ status: "paused" });
    },
    resume: () => {
      send("resume");
      if (get().status === "paused") set({ status: "playing" });
    },
    seek: (seconds) => {
      send({ seek: seconds });
      if (get().status === "playing" || get().status === "finished") set({ status: "paused" });
    },
    setSpeed: (speed) => {
      set({ speed });
      send({ speed });
    },
    setPreset: (preset) => set({ preset }),
    stop: () => {
      token++;
      const info = get().info;
      set({ status: "idle", info: null, frame: null, series: [], injected: [], startedKey: null });
      if (info) void api.controlRun(info.run_id, "stop").catch(() => {});
    },
  };
});

/** Is a run on (starting, playing or paused)? */
export const isActive = (s: RunStatus) => s === "starting" || s === "playing" || s === "paused";
