// Image export (M7.3): the design as one self-contained SVG, drawn from the design itself
// (not a screenshot of the DOM), in the current theme's colours. PNG rasterises the same
// SVG on a canvas (a data: image, which the CSP already allows), so both formats match.
import type { Design, Registry } from "@/bindings";
import { NODE_DOWN, NODE_OVERLOADED, type Frame } from "@/solve/run/frames";
import { forEachMark, MARK_CROSS, MARK_RING, COLOR_DANGER, pushFrame, spacingGapMs, routeLength, type Ring } from "@/solve/run/PacketLayer";
import { assignSlots } from "@/solve/geometry/slots";
import { routeEdge, toPath, bezierPoint, type Rect, type Route } from "@/solve/geometry/routeEdge";
import { chipFits } from "@/solve/edges/CurvedEdge";
import { GLYPHS } from "@/solve/glyphs";
import { kindSpec } from "@/solve/registry";
import { CARD_H, CARD_W } from "@/solve/store";

/** The colours an export uses, read from the theme's CSS variables. */
export type Palette = Record<
  "canvas" | "card" | "foreground" | "muted" | "border" | "note" | "noteInk" | "noteBorder" | "chip" | "danger" | "warning",
  string
> & { category: Record<string, { tile: string; glyph: string }>; req: string[] };

const CATEGORIES = ["edge", "compute", "data", "async", "ops"] as const;

/** The current theme's palette (falls back to the light theme's values). */
export function readPalette(root: Element = document.documentElement): Palette {
  const css = getComputedStyle(root);
  const v = (name: string, fallback: string) => css.getPropertyValue(name).trim() || fallback;
  const card = v("--card", "#fffcf5");
  const category: Palette["category"] = {};
  for (const c of CATEGORIES) {
    const base = v(`--cat-${c}`, "#35689e");
    category[c] = { tile: mix(base, card, 0.14), glyph: v(`--cat-${c}-glyph`, base) };
  }
  return {
    canvas: v("--canvas", "#f1f3f2"),
    card,
    foreground: v("--foreground", "#26302b"),
    muted: v("--muted-foreground", "#5d635b"),
    border: v("--border", "#d9d5ca"),
    note: v("--note", "#f1e6bd"),
    noteInk: v("--note-foreground", "#26302b"),
    noteBorder: v("--note-border", "#d8c98f"),
    chip: v("--muted", "#ece8dc"),
    danger: v("--destructive", "#9c2f27"),
    warning: v("--amber-base", "#c98535"),
    req: [1, 2, 3, 4, 5, 6].map((i) => v(`--req-${i}`, "#2f6fb5")),
    category,
  };
}

/** `a` over `b` at `share` (hex colours; anything else returns `a`). */
export function mix(a: string, b: string, share: number): string {
  const p = (h: string) => (/^#[0-9a-f]{6}$/i.test(h) ? [1, 3, 5].map((i) => parseInt(h.slice(i, i + 2), 16)) : null);
  const x = p(a);
  const y = p(b);
  if (!x || !y) return a;
  return `#${x.map((c, i) => Math.round(c * share + y[i]! * (1 - share)).toString(16).padStart(2, "0")).join("")}`;
}

const esc = (s: string) => s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");
const clip = (s: string, max: number) => (s.length > max ? `${s.slice(0, max - 1)}…` : s);

export type SceneNames = {
  kind: (kind: string) => string;
  flavour: (flavour: string) => string;
  protocol: (protocol: string) => string;
  zone: (kind: string) => string;
};

const PAD = 40;
const FONT = "Inter, 'Segoe UI', system-ui, sans-serif";

/** The design's bounds with the export margin, in flow coordinates (whole pixels). */
export function sceneBounds(design: Design): { x: number; y: number; w: number; h: number } {
  const boxes: Rect[] = [
    ...design.nodes.map((n) => ({ x: n.position.x, y: n.position.y, w: CARD_W, h: CARD_H })),
    ...design.groups.map((g) => ({ x: g.position.x, y: g.position.y, w: g.size.w, h: g.size.h })),
    ...design.notes.map((n) => ({ x: n.position.x, y: n.position.y, w: n.size.w, h: n.size.h })),
  ];
  const minX = boxes.length ? Math.min(...boxes.map((b) => b.x)) - PAD : 0;
  const minY = boxes.length ? Math.min(...boxes.map((b) => b.y)) - PAD : 0;
  const maxX = boxes.length ? Math.max(...boxes.map((b) => b.x + b.w)) + PAD : 400;
  const maxY = boxes.length ? Math.max(...boxes.map((b) => b.y + b.h)) + PAD : 300;
  return { x: Math.round(minX), y: Math.round(minY), w: Math.round(maxX - minX), h: Math.round(maxY - minY) };
}

/** The design as an SVG document (deterministic for the same design, names and palette). */
export function designSvg(design: Design, registry: Registry, names: SceneNames, palette: Palette): string {
  const rects = new Map<string, Rect>(design.nodes.map((n) => [n.id, { x: n.position.x, y: n.position.y, w: CARD_W, h: CARD_H }]));
  const { x: minX, y: minY, w, h } = sceneBounds(design);
  const out: string[] = [];
  out.push(
    `<svg xmlns="http://www.w3.org/2000/svg" width="${w}" height="${h}" viewBox="${Math.round(minX)} ${Math.round(minY)} ${w} ${h}" font-family="${esc(FONT)}">`,
    `<title>${esc(design.name)}</title>`,
    `<defs><marker id="arrow" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="7" markerHeight="7" orient="auto-start-reverse"><path d="M0 0L10 5L0 10z" fill="${palette.muted}"/></marker></defs>`,
    `<rect x="${Math.round(minX)}" y="${Math.round(minY)}" width="${w}" height="${h}" fill="${palette.canvas}"/>`,
  );
  // Zones first (behind everything), outer ones before inner ones.
  const depth = (id: string | null): number => {
    const g = design.groups.find((x) => x.id === id);
    return g ? 1 + depth(g.parent_id) : 0;
  };
  for (const g of [...design.groups].sort((a, b) => depth(a.id) - depth(b.id))) {
    const label = `${names.zone(g.kind)}${g.label.trim() ? ` · ${g.label.trim()}` : ""}`;
    out.push(
      `<g data-zone="${esc(g.id)}"><rect x="${g.position.x}" y="${g.position.y}" width="${g.size.w}" height="${g.size.h}" rx="14" fill="none" stroke="${palette.border}" stroke-width="1.5" stroke-dasharray="6 5"/>`,
      `<text x="${g.position.x + 14}" y="${g.position.y + 22}" font-size="12" font-weight="600" fill="${palette.muted}">${esc(clip(label, 60))}</text></g>`,
    );
  }
  // Connections, routed and spread as on the canvas, with their protocol chips.
  const slots = assignSlots(design.edges, rects);
  for (const e of design.edges) {
    const a = rects.get(e.from);
    const b = rects.get(e.to);
    if (!a || !b) continue;
    const route = routeEdge(a, b, slots.get(e.id));
    const dash = e.protocol === "replicate" ? ` stroke-dasharray="2 4"` : e.protocol === "publish" || e.protocol === "consume" ? ` stroke-dasharray="6 5"` : "";
    const arrow = e.protocol === "replicate" ? "" : ` marker-end="url(#arrow)"`;
    const mid = bezierPoint(route, 0.5);
    const text = names.protocol(e.protocol);
    const cw = Math.max(36, text.length * 6.6 + 14);
    // As on the canvas (F5): a chip that doesn't fit between the cards is left out.
    const fits = chipFits(Math.hypot(route.p3.x - route.p0.x, route.p3.y - route.p0.y), text);
    out.push(
      `<g data-edge="${esc(e.id)}"><path d="${toPath(route)}" fill="none" stroke="${palette.muted}" stroke-width="1.5"${dash}${arrow}/>`,
      ...(fits
        ? [
            `<rect x="${(mid.x - cw / 2).toFixed(1)}" y="${(mid.y - 10).toFixed(1)}" width="${cw.toFixed(1)}" height="20" rx="10" fill="${palette.chip}" stroke="${palette.border}"/>`,
            `<text x="${mid.x.toFixed(1)}" y="${(mid.y + 4).toFixed(1)}" font-size="11" text-anchor="middle" fill="${palette.foreground}" font-family="ui-monospace, 'Cascadia Mono', monospace">${esc(text)}</text>`,
          ]
        : []),
      `</g>`,
    );
  }
  // Cards: the glyph tile, the title, and "product · kind".
  for (const n of design.nodes) {
    const spec = kindSpec(registry, n.kind);
    const cat = palette.category[spec.category] ?? { tile: palette.card, glyph: palette.foreground };
    const product = names.flavour(n.flavour);
    const kind = names.kind(n.kind);
    const title = n.label.trim() || product;
    const sub = n.label.trim() ? `${product} · ${kind}` : kind;
    const { x, y } = n.position;
    out.push(
      `<g data-card="${esc(n.id)}"><rect x="${x}" y="${y}" width="${CARD_W}" height="${CARD_H}" rx="12" fill="${palette.card}" stroke="${palette.border}"/>`,
      `<rect x="${x + 12}" y="${y + 14}" width="36" height="36" rx="7" fill="${cat.tile}"/>`,
      `<g transform="translate(${x + 20} ${y + 22}) scale(0.8333)" fill="none" stroke="${cat.glyph}" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">${GLYPHS[spec.icon] ?? ""}</g>`,
      `<text x="${x + 60}" y="${y + 28}" font-size="14" font-weight="600" fill="${palette.foreground}">${esc(clip(title, 20))}</text>`,
      `<text x="${x + 60}" y="${y + 46}" font-size="11" fill="${palette.muted}">${esc(clip(sub, 26))}</text></g>`,
    );
  }
  // Sticky notes on top, as on the canvas.
  for (const n of design.notes) {
    const lines = n.text.split("\n").flatMap((l) => (l.match(/.{1,28}(\s|$)|.{1,28}/g) ?? [""]).map((s) => s.trim()));
    out.push(
      `<g data-note="${esc(n.id)}"><rect x="${n.position.x}" y="${n.position.y}" width="${n.size.w}" height="${n.size.h}" rx="3" fill="${palette.note}" stroke="${palette.noteBorder}"/>`,
      ...lines
        .slice(0, Math.max(1, Math.floor((n.size.h - 16) / 18)))
        .map((l, i) => `<text x="${n.position.x + 12}" y="${n.position.y + 26 + i * 18}" font-size="14" fill="${palette.noteInk}">${esc(l)}</text>`),
      `</g>`,
    );
  }
  out.push("</svg>");
  return out.join("\n");
}

/** The SVG as PNG bytes, at `scale` × (2 by default, sharp on high-DPI screens). */
export async function svgToPng(svg: string, scale = 2): Promise<Uint8Array> {
  const size = /width="(\d+)" height="(\d+)"/.exec(svg);
  const w = Number(size?.[1] ?? 800);
  const h = Number(size?.[2] ?? 600);
  const img = new Image();
  img.src = `data:image/svg+xml;base64,${btoa(unescape(encodeURIComponent(svg)))}`;
  await img.decode();
  const canvas = document.createElement("canvas");
  canvas.width = Math.round(w * scale);
  canvas.height = Math.round(h * scale);
  const ctx = canvas.getContext("2d");
  if (!ctx) throw new Error("no 2D canvas");
  ctx.scale(scale, scale);
  ctx.drawImage(img, 0, 0, w, h);
  const blob = await new Promise<Blob | null>((r) => canvas.toBlob(r, "image/png"));
  if (!blob) throw new Error("PNG encoding failed");
  return new Uint8Array(await blob.arrayBuffer());
}

// --- Animated export (M7b.2): one frame's moving parts as a display list ------------------

/** Where a run's frames draw: card rects by frame node index, routes by frame edge index. */
export type SceneLayout = {
  bounds: { x: number; y: number; w: number; h: number };
  cards: (Rect | null)[];
  routes: (Route | null)[];
};

/** The layout for frames whose node and edge indices are `nodeIds` and `edgeIds`. */
export function sceneLayout(design: Design, nodeIds: readonly string[], edgeIds: readonly string[]): SceneLayout {
  const rects = new Map<string, Rect>(design.nodes.map((n) => [n.id, { x: n.position.x, y: n.position.y, w: CARD_W, h: CARD_H }]));
  const slots = assignSlots(design.edges, rects);
  const byId = new Map(design.edges.map((e) => [e.id, e]));
  return {
    bounds: sceneBounds(design),
    cards: nodeIds.map((id) => rects.get(id) ?? null),
    routes: edgeIds.map((id) => {
      const e = byId.get(id);
      const a = e && rects.get(e.from);
      const b = e && rects.get(e.to);
      return e && a && b ? routeEdge(a, b, slots.get(e.id)) : null;
    }),
  };
}

/** Prepare a packet ring for a layout: the same spacing per connection as the live canvas. */
export function prepareRing(ring: Ring, layout: SceneLayout) {
  layout.routes.forEach((r, i) => {
    ring.gapMs[i] = r ? spacingGapMs(routeLength(r)) : 0;
  });
}

/** One drawing operation, in flow coordinates (the `hud` line is drawn in pixels). */
export type Op =
  | { t: "dot"; x: number; y: number; r: number; a: number; c: string }
  | { t: "ring"; x: number; y: number; r: number; a: number; c: string }
  | { t: "cross"; x: number; y: number; k: number; a: number; c: string }
  | { t: "pill"; x: number; y: number; text: string; fill: string; stroke: string; ink: string }
  | { t: "hud"; text: string };

/** Words the frame overlay needs, in the user's language. */
export type FrameWords = {
  stat: (util: number, queue: number) => string;
  down: string;
  hud: (f: Frame) => string;
};

const r1 = (n: number) => Math.round(n * 10) / 10;

/**
 * Frame `i`'s display list: push its packets into `ring` at `clock` (the export's own clock,
 * `i * 1000 / fps`, never the screen's), then list every mark, every card's state pill and
 * the HUD line. Deterministic for the same frames, layout, palette and words.
 */
export function frameOps(
  layout: SceneLayout,
  frame: Frame,
  ring: Ring,
  clock: number,
  frameMs: number,
  palette: Palette,
  words: FrameWords,
): Op[] {
  pushFrame(ring, frame, clock, frameMs);
  const ops: Op[] = [];
  forEachMark(ring, layout.routes, clock, (kind, x, y, size, alpha, color) => {
    const c = color === COLOR_DANGER ? palette.danger : palette.req[color % palette.req.length]!;
    const a = Math.round(alpha * 100) / 100;
    if (kind === MARK_CROSS) ops.push({ t: "cross", x: r1(x), y: r1(y), k: r1(size), a, c });
    else if (kind === MARK_RING) ops.push({ t: "ring", x: r1(x), y: r1(y), r: r1(size), a, c });
    else ops.push({ t: "dot", x: r1(x), y: r1(y), r: r1(size), a, c });
  });
  layout.cards.forEach((rect, i) => {
    if (!rect || i >= frame.util.length) return;
    const flags = frame.nodeFlags[i] ?? 0;
    const down = (flags & NODE_DOWN) !== 0;
    const over = (flags & NODE_OVERLOADED) !== 0;
    // Idle cards (clients, unused stores) get no pill: it would only say 0%.
    if (!down && frame.util[i] === 0 && frame.queue[i] === 0) return;
    ops.push({
      t: "pill",
      // On the card's top-right corner: clear of the connections and their chips.
      x: rect.x + rect.w - 34,
      y: rect.y,
      text: down ? words.down : words.stat(frame.util[i]!, frame.queue[i]!),
      fill: palette.card,
      stroke: down ? palette.danger : over ? palette.warning : palette.border,
      ink: down ? palette.danger : palette.foreground,
    });
  });
  ops.push({ t: "hud", text: words.hud(frame) });
  return ops;
}

/** The HUD band's height under the scene (pixels). */
export const HUD_H = 36;

/** Paint a display list: `scale` maps flow coordinates to pixels; the HUD sits at the bottom. */
export function paintOps(ctx: CanvasRenderingContext2D, ops: readonly Op[], layout: SceneLayout, scale: number, palette: Palette, offsetX = 0) {
  const { x: bx, y: by } = layout.bounds;
  ctx.setTransform(scale, 0, 0, scale, offsetX - bx * scale, -by * scale);
  for (const op of ops) {
    if (op.t === "hud") continue;
    ctx.globalAlpha = "a" in op ? op.a : 1;
    if (op.t === "dot") {
      ctx.fillStyle = op.c;
      ctx.beginPath();
      ctx.arc(op.x, op.y, op.r, 0, Math.PI * 2);
      ctx.fill();
    } else if (op.t === "ring") {
      ctx.strokeStyle = op.c;
      ctx.lineWidth = 1.5;
      ctx.beginPath();
      ctx.arc(op.x, op.y, op.r, 0, Math.PI * 2);
      ctx.stroke();
    } else if (op.t === "cross") {
      ctx.strokeStyle = op.c;
      ctx.lineWidth = 2;
      ctx.beginPath();
      ctx.moveTo(op.x - op.k, op.y - op.k);
      ctx.lineTo(op.x + op.k, op.y + op.k);
      ctx.moveTo(op.x + op.k, op.y - op.k);
      ctx.lineTo(op.x - op.k, op.y + op.k);
      ctx.stroke();
    } else {
      ctx.font = `600 11px ${FONT}`;
      const w = Math.max(40, ctx.measureText(op.text).width + 16);
      ctx.fillStyle = op.fill;
      ctx.strokeStyle = op.stroke;
      ctx.lineWidth = 1.5;
      ctx.beginPath();
      ctx.roundRect(op.x - w / 2, op.y - 9, w, 18, 9);
      ctx.fill();
      ctx.stroke();
      ctx.fillStyle = op.ink;
      ctx.textAlign = "center";
      ctx.fillText(op.text, op.x, op.y + 4);
    }
  }
  ctx.globalAlpha = 1;
  const hud = ops.find((o) => o.t === "hud");
  if (hud && hud.t === "hud") {
    ctx.setTransform(1, 0, 0, 1, 0, 0);
    const height = ctx.canvas.height;
    ctx.fillStyle = palette.card;
    ctx.fillRect(0, height - HUD_H, ctx.canvas.width, HUD_H);
    ctx.fillStyle = palette.foreground;
    ctx.font = `500 14px ${FONT}`;
    ctx.textAlign = "left";
    ctx.fillText(hud.text, 16, height - HUD_H / 2 + 5);
  }
}

/** Load an SVG document as an image (a data: URL, which the CSP already allows). */
export async function svgImage(svg: string): Promise<HTMLImageElement> {
  const img = new Image();
  img.src = `data:image/svg+xml;base64,${btoa(unescape(encodeURIComponent(svg)))}`;
  await img.decode();
  return img;
}
