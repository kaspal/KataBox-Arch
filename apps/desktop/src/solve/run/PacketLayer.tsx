// The packet layer (plan M1.4): one <canvas> over the React Flow pane, drawn in flow
// coordinates with the viewport's transform. Each packet in a frame becomes a dot that
// rides its connection's curve (the same slotted route the edge draws); async messages get
// a comet tail; a cache hit is a ring at the far end; a drop is a cross in the danger colour.
//
// - Packets live in a fixed ring of typed arrays: nothing is allocated per packet.
// - The animation clock only advances while the run plays and the window is visible, so a
//   pause freezes the picture and a resume carries on from it.
// - With reduced motion there are no packets (connections show their throughput instead).
import { useEffect, useRef } from "react";
import { useStoreApi } from "@xyflow/react";
import { useReducedMotion } from "@/lib/motion";
import { nodeRect, type CurvedFlowEdge } from "@/solve/edges/CurvedEdge";
import { routeEdge, type Route } from "@/solve/geometry/routeEdge";
import { onFrame, useRun } from "@/solve/run/runStore";
import { PACKET_ASYNC, PACKET_DROP, PACKET_HIT, PACKET_REVERSE, type Frame } from "@/solve/run/frames";

/** Packets the ring holds (about two seconds at the frame cap). */
export const RING = 4096;
/** Wall-clock ms a hop takes on screen, and a hit or drop mark lasts. */
export const HOP_MS = 600;
export const EVENT_MS = 450;
/** Frames arrive about every 33 ms: a frame's packets start spread over that span. */
const FRAME_MS = 33;
const RADIUS = 3.5;
/** Marks drawn per frame: a busy cache hits hundreds of times a second, and its rings would
 * pile up into a blob. A few per frame keep the sparkle; drops get more room. */
export const MAX_HITS_PER_FRAME = 2;
export const MAX_DROPS_PER_FRAME = 6;

/** The packets of one frame, appended to the ring (exported for the dev perf page's test). */
export type Ring = {
  edge: Int32Array;
  type: Uint8Array;
  start: Float64Array;
  dur: Float32Array;
  head: number;
};

export function newRing(): Ring {
  return {
    edge: new Int32Array(RING).fill(-1),
    type: new Uint8Array(RING),
    start: new Float64Array(RING),
    dur: new Float32Array(RING),
    head: 0,
  };
}

export function pushFrame(ring: Ring, f: Frame, clock: number) {
  if (f.windowMs <= 0) return;
  let hits = 0;
  let drops = 0;
  for (let i = 0; i < f.packetCount; i++) {
    const t = f.packetType[i];
    if (t & PACKET_HIT && ++hits > MAX_HITS_PER_FRAME) continue;
    if (t & PACKET_DROP && ++drops > MAX_DROPS_PER_FRAME) continue;
    const at = ring.head;
    ring.edge[at] = f.packetEdge[i];
    ring.type[at] = t;
    ring.start[at] = clock + (f.packetStart[i] / f.windowMs) * FRAME_MS;
    ring.dur[at] = t & (PACKET_HIT | PACKET_DROP) ? EVENT_MS : HOP_MS;
    ring.head = (at + 1) % RING;
  }
}

/** How many packets are on screen at `clock`. */
export function visible(ring: Ring, clock: number): number {
  let n = 0;
  for (let i = 0; i < RING; i++) {
    if (ring.edge[i] >= 0 && clock >= ring.start[i] && clock < ring.start[i] + ring.dur[i]) n++;
  }
  return n;
}

const ease = (t: number) => (t < 0.5 ? 2 * t * t : 1 - (-2 * t + 2) ** 2 / 2);

export function PacketLayer() {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const store = useStoreApi();
  const reduced = useReducedMotion();

  useEffect(() => {
    const canvas = canvasRef.current;
    const ctx = canvas?.getContext("2d");
    if (!canvas || !ctx || reduced) return;
    const ring = newRing();
    let clock = 0;
    let last = performance.now();
    let colors: string[] = [];
    let danger = "#c00";
    const readColors = () => {
      const cs = getComputedStyle(document.documentElement);
      colors = [1, 2, 3, 4, 5, 6].map((i) => cs.getPropertyValue(`--req-${i}`).trim() || "#888");
      danger = cs.getPropertyValue("--destructive").trim() || danger;
    };
    readColors();
    const themeWatch = new MutationObserver(readColors);
    themeWatch.observe(document.documentElement, { attributes: true, attributeFilter: ["class"] });

    const offFrame = onFrame((f) => pushFrame(ring, f, clock));

    // Routes by frame edge index, rebuilt when the edge list or a card moves.
    let routes: (Route | null)[] = [];
    let edgesRef: unknown = null;
    let lookupRef: unknown = null;
    let edgeIdsRef: string[] | null = null;
    const ensureRoutes = () => {
      const s = store.getState();
      const ids = useRun.getState().info?.edges ?? null;
      if (s.edges === edgesRef && s.nodeLookup === lookupRef && ids === edgeIdsRef) return;
      edgesRef = s.edges;
      lookupRef = s.nodeLookup;
      edgeIdsRef = ids;
      const byId = new Map((s.edges as CurvedFlowEdge[]).map((e) => [e.id, e]));
      routes = (ids ?? []).map((id) => {
        const e = byId.get(id);
        const a = e && s.nodeLookup.get(e.source);
        const b = e && s.nodeLookup.get(e.target);
        return e && a && b ? routeEdge(nodeRect(a), nodeRect(b), e.data?.slots) : null;
      });
    };

    let cleared = true;
    const draw = () => {
      const dpr = window.devicePixelRatio || 1;
      const w = canvas.clientWidth;
      const h = canvas.clientHeight;
      if (canvas.width !== Math.round(w * dpr) || canvas.height !== Math.round(h * dpr)) {
        canvas.width = Math.round(w * dpr);
        canvas.height = Math.round(h * dpr);
      }
      ctx.setTransform(1, 0, 0, 1, 0, 0);
      ctx.clearRect(0, 0, canvas.width, canvas.height);
      ensureRoutes();
      const [tx, ty, zoom] = store.getState().transform;
      ctx.setTransform(dpr * zoom, 0, 0, dpr * zoom, dpr * tx, dpr * ty);
      for (let i = 0; i < RING; i++) {
        const e = ring.edge[i];
        if (e < 0) continue;
        const s = ring.start[i];
        const d = ring.dur[i];
        if (clock < s || clock >= s + d) continue;
        const r = routes[e];
        if (!r) continue;
        const ty8 = ring.type[i];
        const t = (clock - s) / d;
        const rev = (ty8 & PACKET_REVERSE) !== 0;
        if (ty8 & (PACKET_HIT | PACKET_DROP)) {
          // Marks sit where the packet arrives.
          const p = rev ? r.p0 : r.p3;
          ctx.globalAlpha = ty8 & PACKET_DROP ? 1 - t : 0.6 * (1 - t);
          if (ty8 & PACKET_DROP) {
            const k = 3 + 7 * t;
            ctx.strokeStyle = danger;
            ctx.lineWidth = 2;
            ctx.beginPath();
            ctx.moveTo(p.x - k, p.y - k);
            ctx.lineTo(p.x + k, p.y + k);
            ctx.moveTo(p.x + k, p.y - k);
            ctx.lineTo(p.x - k, p.y + k);
            ctx.stroke();
          } else {
            ctx.strokeStyle = colors[(ty8 & 7) % colors.length];
            ctx.lineWidth = 1.5;
            ctx.beginPath();
            ctx.arc(p.x, p.y, 4 + 9 * t, 0, Math.PI * 2);
            ctx.stroke();
          }
          continue;
        }
        ctx.fillStyle = colors[(ty8 & 7) % colors.length];
        const tail = ty8 & PACKET_ASYNC ? 3 : 1;
        for (let k = 0; k < tail; k++) {
          let u = ease(Math.max(0, t - k * 0.06));
          if (rev) u = 1 - u;
          const v = 1 - u;
          // The cubic Bézier, inline (no objects per packet).
          const x = v * v * v * r.p0.x + 3 * v * v * u * r.p1.x + 3 * v * u * u * r.p2.x + u * u * u * r.p3.x;
          const y = v * v * v * r.p0.y + 3 * v * v * u * r.p1.y + 3 * v * u * u * r.p2.y + u * u * u * r.p3.y;
          ctx.globalAlpha = k === 0 ? 1 : 0.45 / k;
          ctx.beginPath();
          ctx.arc(x, y, k === 0 ? RADIUS : RADIUS * 0.8, 0, Math.PI * 2);
          ctx.fill();
        }
      }
      ctx.globalAlpha = 1;
      cleared = false;
    };

    let raf = 0;
    const tick = (now: number) => {
      raf = requestAnimationFrame(tick);
      const dt = Math.min(now - last, 100);
      last = now;
      const status = useRun.getState().status;
      if (status === "idle" || status === "starting") {
        if (!cleared) {
          ctx.setTransform(1, 0, 0, 1, 0, 0);
          ctx.clearRect(0, 0, canvas.width, canvas.height);
          ring.edge.fill(-1);
          cleared = true;
        }
        return;
      }
      // Paused, finished or hidden: the picture stays as it is.
      if (status !== "playing" || document.hidden) return;
      clock += dt;
      draw();
    };
    raf = requestAnimationFrame(tick);
    return () => {
      cancelAnimationFrame(raf);
      offFrame();
      themeWatch.disconnect();
    };
  }, [store, reduced]);

  if (reduced) return null;
  return <canvas ref={canvasRef} aria-hidden className="pointer-events-none absolute inset-0 z-[4] size-full" data-testid="packet-layer" />;
}
