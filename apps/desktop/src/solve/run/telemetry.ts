// What the canvas shows of a live run besides packets: each card's health (utilisation,
// queue, down, overloaded), refreshed at most 10 times a second, and each connection's
// throughput (real requests per second, smoothed) for the reduced-motion labels.
import { create } from "zustand";
import { NODE_DOWN, NODE_OVERLOADED, PACKET_DROP, PACKET_HIT, type Frame } from "@/solve/run/frames";
import { onFrame, useRun } from "@/solve/run/runStore";

export type NodeHealth = { util: number; queue: number; down: boolean; overloaded: boolean };

type Telemetry = {
  byId: Record<string, NodeHealth>;
  /** Requests per second on each connection, by edge id. */
  edgeRates: Record<string, number>;
};

export const useTelemetry = create<Telemetry>()(() => ({ byId: {}, edgeRates: {} }));

const FLUSH_MS = 100;
/** Smoothing of per-edge rates per frame (about a third of a second at 30 frames/s). */
const EMA = 0.1;

const same = (a: NodeHealth | undefined, b: NodeHealth) =>
  !!a && a.util === b.util && a.queue === b.queue && a.down === b.down && a.overloaded === b.overloaded;

/** Listen to the run's frames; returns a function that stops and clears. */
export function startTelemetry(): () => void {
  let rates: number[] = [];
  let pending: Frame | null = null;
  let timer: ReturnType<typeof setTimeout> | undefined;

  const flush = () => {
    timer = undefined;
    const f = pending;
    const info = useRun.getState().info;
    if (!f || !info) return;
    const prev = useTelemetry.getState();
    const byId: Record<string, NodeHealth> = {};
    info.nodes.forEach((id, i) => {
      const h: NodeHealth = {
        util: f.util[i] ?? 0,
        queue: f.queue[i] ?? 0,
        down: ((f.nodeFlags[i] ?? 0) & NODE_DOWN) !== 0,
        overloaded: ((f.nodeFlags[i] ?? 0) & NODE_OVERLOADED) !== 0,
      };
      // Keep unchanged objects, so a card re-renders only when its numbers move.
      byId[id] = same(prev.byId[id], h) ? prev.byId[id] : h;
    });
    const edgeRates: Record<string, number> = {};
    info.edges.forEach((id, i) => {
      edgeRates[id] = rates[i] ?? 0;
    });
    useTelemetry.setState({ byId, edgeRates });
  };

  const off = onFrame((f) => {
    const info = useRun.getState().info;
    if (!info) return;
    if (f.windowMs > 0) {
      // Packets are a thinned sample of the simulated hops; scale back to real traffic.
      const scale = f.packetCount > 0 ? f.packetTotal / f.packetCount : 0;
      const perSecond = scale / (f.windowMs / 1000) / Math.max(info.sample_factor, 1e-9);
      const counts = new Array<number>(info.edges.length).fill(0);
      for (let i = 0; i < f.packetCount; i++) {
        if (f.packetType[i] & (PACKET_HIT | PACKET_DROP)) continue;
        const e = f.packetEdge[i];
        if (e < counts.length) counts[e] += 1;
      }
      rates = counts.map((c, i) => {
        const prev = rates[i] ?? 0;
        return prev + EMA * (c * perSecond - prev);
      });
    }
    pending = f;
    timer ??= setTimeout(flush, FLUSH_MS);
  });

  const unsub = useRun.subscribe((s, prev) => {
    if (s.status === "idle" && prev.status !== "idle") {
      rates = [];
      pending = null;
      useTelemetry.setState({ byId: {}, edgeRates: {} });
    }
  });

  return () => {
    off();
    unsub();
    clearTimeout(timer);
    useTelemetry.setState({ byId: {}, edgeRates: {} });
  };
}
