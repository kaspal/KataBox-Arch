// The live run's state, outside the editor's undo history: status, the latest frame and
// the controls. Rust owns the run (ka-app/src/live.rs); this store only mirrors it.
import { create } from "zustand";
import type { Design, RunInfo, SandboxPreset } from "@/bindings";
import { api } from "@/lib/api";
import { decodeFrame, type Frame } from "@/solve/run/frames";

export type RunStatus = "idle" | "starting" | "playing" | "paused" | "finished";

export const SPEEDS = [0.25, 0.5, 1, 2, 4, 8] as const;

type RunState = {
  status: RunStatus;
  info: RunInfo | null;
  /** The latest frame; kept while an edited design restarts, until the new run's first frame. */
  frame: Frame | null;
  preset: SandboxPreset;
  speed: number;
  /** What the running design looked like to the simulator (see `simKey`). */
  startedKey: string | null;
  start: (design: Design) => Promise<void>;
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

// Each start (or stop) bumps the token; frames and answers of older runs are ignored.
let token = 0;
const frameListeners = new Set<(f: Frame) => void>();

/** Called on every frame (the packet layer, M1.4). Returns an unsubscribe function. */
export function onFrame(cb: (f: Frame) => void): () => void {
  frameListeners.add(cb);
  return () => frameListeners.delete(cb);
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
    preset: "normal",
    speed: 1,
    startedKey: null,

    start: async (design) => {
      const mine = ++token;
      set({ status: "starting", startedKey: simKey(design) });
      try {
        const info = await api.startRun(design, get().preset, get().speed, (buf) => {
          if (mine !== token) return;
          let f: Frame;
          try {
            f = decodeFrame(buf);
          } catch {
            return;
          }
          set({ frame: f, status: f.finished ? "finished" : f.playing ? "playing" : "paused" });
          frameListeners.forEach((l) => l(f));
        });
        if (mine === token) set({ info });
      } catch (e) {
        if (mine === token) set({ status: "idle", info: null, startedKey: null });
        throw e;
      }
    },
    pause: () => {
      send("pause");
      if (get().status === "playing") set({ status: "paused" });
    },
    resume: () => {
      send("resume");
      if (get().status === "paused") set({ status: "playing" });
    },
    seek: (seconds) => send({ seek: seconds }),
    setSpeed: (speed) => {
      set({ speed });
      send({ speed });
    },
    setPreset: (preset) => set({ preset }),
    stop: () => {
      token++;
      const info = get().info;
      set({ status: "idle", info: null, frame: null, startedKey: null });
      if (info) void api.controlRun(info.run_id, "stop").catch(() => {});
    },
  };
});

/** Hand a frame to the listeners without a run behind it (the dev perf page). */
export function emitFrame(f: Frame) {
  frameListeners.forEach((l) => l(f));
}

/** Is a run on (starting, playing or paused)? */
export const isActive = (s: RunStatus) => s === "starting" || s === "playing" || s === "paused";
