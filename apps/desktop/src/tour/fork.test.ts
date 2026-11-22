import { describe, expect, it } from "vitest";
import type { Design } from "@/bindings";
import { forkDesign } from "@/tour/fork";

const D: Design = {
  schema_version: 2,
  id: "s1",
  name: "Sample",
  problem_id: "url-shortener",
  nodes: [
    { id: "a", kind: "app", flavour: "go", label: "", position: { x: 0, y: 0 }, props: { instances: 2 }, group_id: "z" },
    { id: "b", kind: "sql", flavour: "postgresql", label: "DB", position: { x: 0, y: 200 }, props: {}, group_id: null },
  ],
  edges: [{ id: "e", from: "a", to: "b", protocol: "query" }],
  groups: [
    { id: "r", kind: "region", label: "EU", position: { x: -50, y: -50 }, size: { w: 600, h: 500 }, parent_id: null },
    { id: "z", kind: "az", label: "A", position: { x: -20, y: -20 }, size: { w: 300, h: 200 }, parent_id: "r" },
  ],
  notes: [{ id: "n", text: "hi", position: { x: 400, y: 0 }, size: { w: 160, h: 80 } }],
  viewport: { x: 0, y: 0, zoom: 1 },
};

describe("forking a snapshot", () => {
  it("renews every id, keeps every reference and drops the problem", () => {
    const f = forkDesign(D, "My copy");
    expect(f.name).toBe("My copy");
    expect(f.problem_id).toBeNull();
    const old = new Set(["s1", "a", "b", "e", "r", "z", "n"]);
    for (const id of [f.id, ...f.nodes.map((x) => x.id), ...f.edges.map((x) => x.id), ...f.groups.map((x) => x.id), ...f.notes.map((x) => x.id)]) {
      expect(old.has(id)).toBe(false);
    }
    const [a, b] = f.nodes;
    const [r, z] = f.groups;
    expect(f.edges[0]).toMatchObject({ from: a.id, to: b.id, protocol: "query" });
    expect(a.group_id).toBe(z.id);
    expect(b.group_id).toBeNull();
    expect(z.parent_id).toBe(r.id);
    expect(a.props).toEqual({ instances: 2 });
    // The original is untouched.
    expect(D.nodes[0].id).toBe("a");
  });
});
