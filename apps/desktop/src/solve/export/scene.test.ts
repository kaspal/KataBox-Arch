import { describe, expect, it } from "vitest";
import type { Design, Registry } from "@/bindings";
import fixture from "@/test/fixtures/registry.json";
import { designSvg, mix, type Palette, type SceneNames } from "@/solve/export/scene";

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
