import { describe, expect, it } from "vitest";
import type { Design, Registry } from "@/bindings";
import fixture from "@/test/fixtures/registry.json";
import { designSvg, frameOps, mix, prepareRing, sceneLayout, type FrameWords, type Palette, type SceneNames } from "@/solve/export/scene";
import { newRing } from "@/solve/run/PacketLayer";
import { NODE_DOWN, NODE_OVERLOADED, PACKET_ASYNC, PACKET_DROP, PACKET_HIT, type Frame } from "@/solve/run/frames";

const registry = fixture.registry as unknown as Registry;
const palette: Palette = {
  canvas: "#f1f3f2",
  card: "#fffcf5",
  foreground: "#26302b",
  muted: "#5d635b",
  border: "#d9d5ca",
  note: "#f1e6bd",
  noteInk: "#26302b",
  noteBorder: "#d8c98f",
  chip: "#ece8dc",
  danger: "#9c2f27",
  warning: "#c98535",
  req: ["#2f6fb5", "#b2541a", "#7a4fb0", "#1f7a72", "#b0306f", "#5d635b"],
  category: Object.fromEntries(["edge", "compute", "data", "async", "ops"].map((c) => [c, { tile: "#e0e8f0", glyph: "#35689e" }])),
};
const names: SceneNames = { kind: (k) => `K:${k}`, flavour: (f) => `F:${f}`, protocol: (p) => p.toUpperCase(), zone: (z) => `Z:${z}` };

const design: Design = {
  schema_version: 3,
  id: "d",
  name: "Shop <beta>",
  problem_id: null,
  nodes: [
    { id: "c1", kind: "client", flavour: "web", label: "", position: { x: 0, y: 0 }, props: {}, group_id: null },
    { id: "a1", kind: "app", flavour: "go", label: "API & \"admin\"", position: { x: 0, y: 200 }, props: {}, group_id: "r1" },
  ],
  edges: [{ id: "e1", from: "c1", to: "a1", protocol: "http" }],
  groups: [{ id: "r1", kind: "region", label: "eu-west", position: { x: -40, y: 160 }, size: { w: 290, h: 150 }, parent_id: null }],
  notes: [{ id: "n1", text: "Remember the cache", position: { x: 300, y: 0 }, size: { w: 180, h: 80 } }],
  viewport: { x: 0, y: 0, zoom: 1 },
};

describe("image export scene", () => {
  it("draws zones, connections, cards and notes as one deterministic SVG", () => {
    const svg = designSvg(design, registry, names, palette);
    expect(svg.startsWith('<svg xmlns="http://www.w3.org/2000/svg"')).toBe(true);
    expect(svg).toContain("<title>Shop &lt;beta&gt;</title>");
    expect(svg).toContain("Z:region · eu-west");
    expect(svg).toContain(">HTTP</text>");
    expect(svg).toContain(">F:web</text>");
    expect(svg).toContain(">K:client</text>");
    // Labels are escaped: no text can become markup.
    expect(svg).toContain("API &amp; &quot;admin&quot;");
    expect(svg).toContain("Remember the cache");
    // Painting order: zones, then edges, then cards, then notes.
    const order = ["data-zone", "data-edge", "data-card", "data-note"].map((k) => svg.indexOf(k));
    expect([...order].sort((a, b) => a - b)).toEqual(order);
    expect(designSvg(design, registry, names, palette)).toBe(svg);
    // The canvas covers every box plus a margin.
    expect(svg).toMatch(/viewBox="-80 -40 \d+ \d+"/);
  });

  it("mixes theme colours for the glyph tiles", () => {
    expect(mix("#ff0000", "#0000ff", 0.25)).toBe("#4000bf");
    expect(mix("var(--x)", "#000000", 0.5)).toBe("var(--x)");
  });
});

describe("the animated export's display list (M7b.2)", () => {
  const design: Design = {
    schema_version: 3,
    id: "anim",
    name: "Anim",
    problem_id: null,
    nodes: [
      { id: "lb", kind: "lb", flavour: "nginx", label: "", position: { x: 0, y: 0 }, props: {}, group_id: null },
      { id: "app", kind: "app", flavour: "go", label: "", position: { x: 320, y: 0 }, props: {}, group_id: null },
      { id: "db", kind: "sql", flavour: "postgresql", label: "", position: { x: 640, y: 0 }, props: {}, group_id: null },
    ],
    edges: [
      { id: "e1", from: "lb", to: "app", protocol: "http" },
      { id: "e2", from: "app", to: "db", protocol: "query" },
    ],
    groups: [],
    notes: [],
    viewport: { x: 0, y: 0, zoom: 1 },
  };
  const frame = (i: number): Frame => ({
    frame: i,
    tMs: 1000 + i * 33,
    windowMs: 33,
    playing: true,
    finished: false,
    speed: 1,
    p95Ms: 31.2,
    rps: 1200,
    errorRate: i === 2 ? 0.05 : 0,
    costMonth: 900,
    staleRate: 0,
    retried: 0,
    util: Uint8Array.from([20, 40 + i * 20, 97]),
    queue: Uint16Array.from([0, 0, 12]),
    nodeFlags: Uint8Array.from([0, 0, i === 2 ? NODE_DOWN : NODE_OVERLOADED]),
    packetCount: 4,
    packetTotal: 4,
    packetEdge: Uint16Array.from([0, 1, 1, 0]),
    packetType: Uint8Array.from([0, 1 | PACKET_ASYNC, PACKET_HIT, PACKET_DROP]),
    packetStart: Uint16Array.from([0, 8, 16, 30]),
    packetTravel: Uint16Array.from([10, 10, 0, 0]),
  });
  const words: FrameWords = {
    stat: (u, q) => (q > 0 ? `${u}% · q${q}` : `${u}%`),
    down: "down",
    hud: (f) => `t ${(f.tMs / 1000).toFixed(1)} s · p95 ${f.p95Ms} ms`,
  };

  it("lists every packet mark, card state and the HUD, deterministically", async () => {
    const run = () => {
      const layout = sceneLayout(design, ["lb", "app", "db"], ["e1", "e2"]);
      const ring = newRing();
      prepareRing(ring, layout);
      return [0, 1, 2].map((i) => frameOps(layout, frame(i), ring, i * 1000 / 30 + 200 * i, 1000 / 30, palette, words));
    };
    const a = run();
    expect(JSON.stringify(a)).toBe(JSON.stringify(run()));
    const last = a[2]!;
    expect(last.filter((o) => o.t === "dot").length).toBeGreaterThan(0);
    expect(last.some((o) => o.t === "cross")).toBe(true);
    expect(last.find((o) => o.t === "pill" && o.text === "down")).toMatchObject({ stroke: "#9c2f27" });
    expect(a[1]!.find((o) => o.t === "pill" && o.text === "97% · q12")).toMatchObject({ stroke: "#c98535" });
    expect(last.at(-1)).toEqual({ t: "hud", text: "t 1.1 s · p95 31.2 ms" });
    await expect(JSON.stringify(a, null, 1)).toMatchFileSnapshot("./__golden__/frameops.json");
  });
});
