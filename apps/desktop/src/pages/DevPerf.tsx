// /dev/perf (development builds only): the packet layer's frame rate on a 50-card design with
// about 2,000 packets on screen, fed synthetic frames at 30 Hz. The number goes in the report.
import { useEffect, useRef, useState } from "react";
import type { Design, Registry } from "@/bindings";
import { useRegistry } from "@/lib/queries";
import { Canvas } from "@/solve/Canvas";
import type { Frame } from "@/solve/run/frames";
import { emitFrame, useRun } from "@/solve/run/runStore";
import { startTelemetry } from "@/solve/run/telemetry";
import { useEditor } from "@/solve/store";

const COLS = 10;
const ROWS = 5;
/** Packets per frame: ~110 × (600 ms on screen / 33 ms per frame) ≈ 2,000 visible. */
const PER_FRAME = 110;

function perfDesign(): Design {
  const kinds = ["app", "sql", "cache", "lb", "worker"] as const;
  const flavours = { app: "go", sql: "postgresql", cache: "redis", lb: "nginx", worker: "celery" } as const;
  const nodes: Design["nodes"] = [];
  for (let r = 0; r < ROWS; r++) {
    for (let c = 0; c < COLS; c++) {
      const kind = kinds[(r + c) % kinds.length];
      nodes.push({
        id: `n${r}-${c}`,
        kind,
        flavour: flavours[kind],
        label: "",
        position: { x: c * 260, y: r * 170 },
        props: {},
        group_id: null,
      });
    }
  }
  const edges: Design["edges"] = [];
  for (let r = 0; r < ROWS; r++) {
    for (let c = 0; c < COLS; c++) {
      if (c + 1 < COLS) edges.push({ id: `h${r}-${c}`, from: `n${r}-${c}`, to: `n${r}-${c + 1}`, protocol: "http" });
      if (r + 1 < ROWS && c % 3 === 0) edges.push({ id: `v${r}-${c}`, from: `n${r}-${c}`, to: `n${r + 1}-${c}`, protocol: "query" });
    }
  }
  return { schema_version: 2, id: "perf", name: "Perf", problem_id: null, nodes, edges, groups: [], notes: [], viewport: { x: 0, y: 0, zoom: 0.5 } };
}

function synthetic(n: number, edges: number, seq: number): Frame {
  // A small LCG: the same picture every time, no Math.random.
  let s = seq * 2654435761;
  const next = () => (s = (s * 1103515245 + 12345) >>> 0) / 2 ** 32;
  const util = Uint8Array.from({ length: n }, (_, i) => Math.round(50 + 45 * Math.sin(seq / 30 + i)));
  return {
    frame: seq,
    tMs: seq * 33,
    windowMs: 33,
    playing: true,
    finished: false,
    speed: 1,
    p95Ms: 42,
    rps: 50_000,
    errorRate: 0,
    costMonth: 9_000,
    util,
    queue: new Uint16Array(n),
    nodeFlags: new Uint8Array(n),
    packetCount: PER_FRAME,
    packetTotal: PER_FRAME,
    packetEdge: Uint16Array.from({ length: PER_FRAME }, () => Math.floor(next() * edges)),
    packetType: Uint8Array.from({ length: PER_FRAME }, () => Math.floor(next() * 3) | (next() < 0.2 ? 8 : 0)),
    packetStart: Uint16Array.from({ length: PER_FRAME }, () => Math.floor(next() * 33)),
    packetTravel: new Uint16Array(PER_FRAME),
  };
}

function Bench({ registry }: { registry: Registry }) {
  const [fps, setFps] = useState<number | null>(null);
  const frames = useRef<number[]>([]);
  useEffect(() => {
    const d = perfDesign();
    useEditor.getState().load(d, null);
    useRun.setState({
      status: "playing",
      info: { run_id: 0, duration_s: 600, sample_factor: 1, nodes: d.nodes.map((n) => n.id), edges: d.edges.map((e) => e.id), request_types: ["a", "b", "c"] },
    });
    const stopTelemetry = startTelemetry();
    let seq = 0;
    const feed = setInterval(() => {
      const f = synthetic(d.nodes.length, d.edges.length, ++seq);
      useRun.setState({ frame: f });
      emitFrame(f);
    }, 33);
    let raf = 0;
    const tick = (t: number) => {
      raf = requestAnimationFrame(tick);
      const list = frames.current;
      list.push(t);
      while (list.length && t - list[0] > 2000) list.shift();
    };
    raf = requestAnimationFrame(tick);
    const report = setInterval(() => {
      const list = frames.current;
      if (list.length > 1) setFps(Math.round(((list.length - 1) * 1000) / (list[list.length - 1] - list[0])));
    }, 500);
    return () => {
      clearInterval(feed);
      clearInterval(report);
      cancelAnimationFrame(raf);
      stopTelemetry();
      useRun.getState().stop();
    };
  }, []);
  return (
    <div className="relative h-full">
      <Canvas registry={registry} />
      <div className="absolute left-4 top-4 z-10 rounded-md border bg-card px-3 py-2 font-mono text-sm" data-testid="perf-fps">
        {`${COLS * ROWS} cards · ~2,000 packets · ${fps ?? "…"} fps`}
      </div>
    </div>
  );
}

export function DevPerf() {
  const registry = useRegistry();
  return <main className="h-full">{registry.data ? <Bench registry={registry.data} /> : null}</main>;
}
