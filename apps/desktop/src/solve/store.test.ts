import { beforeEach, describe, expect, it } from "vitest";
import { REGISTRY } from "@/test/fakeApi";
import { GLYPHS } from "@/solve/glyphs";
import { docJson, fromDesign, isDirty, redo, toDesign, undo, useEditor, withMembership } from "@/solve/store";

const s = () => useEditor.getState();
const past = () => useEditor.temporal.getState().pastStates.length;

beforeEach(() => {
  s().newDesign("Test");
});

describe("editor store", () => {
  it("new cards take the kind's defaults with the flavour's overrides", () => {
    const id = s().addNode(REGISTRY, "app", "go", { x: 0, y: 0 });
    const node = s().doc.nodes.find((n) => n.id === id)!;
    expect(node.props.instances).toBe(2);
    expect(node.props.service_ms).toBe(5); // Go overrides the default 10
    expect(id).toMatch(/^[A-Za-z0-9_-]{1,64}$/);
  });

  it("a flavour change moves untouched props to the new defaults and keeps edited ones", () => {
    const id = s().addNode(REGISTRY, "app", "nodejs", { x: 0, y: 0 });
    s().setProp(id, "instances", 5);
    s().setFlavour(REGISTRY, id, "django");
    const node = s().doc.nodes.find((n) => n.id === id)!;
    expect(node.props.service_ms).toBe(20);
    expect(node.props.instances).toBe(5);
  });

  it("round-trips through the file shape", () => {
    const a = s().addNode(REGISTRY, "client", "web", { x: 10, y: 20 });
    const b = s().addNode(REGISTRY, "lb", "nginx", { x: 300, y: 20 });
    s().addEdge(a, b, "http");
    s().addNote({ x: 0, y: 200 });
    const design = toDesign(s());
    expect(design.schema_version).toBe(1);
    expect(fromDesign(design)).toEqual(s().doc);
    s().load(design, "C:/d.kbarch");
    expect(isDirty()).toBe(false);
    expect(past()).toBe(0);
  });

  it("is dirty after an edit and clean after markSaved", () => {
    expect(isDirty()).toBe(false);
    s().addNote({ x: 0, y: 0 });
    expect(isDirty()).toBe(true);
    s().markSaved("C:/x.kbarch");
    expect(isDirty()).toBe(false);
    expect(s().savedJson).toBe(docJson(s().doc));
  });

  it("removing a card removes its edges and zone members fall out of a removed zone", () => {
    const zone = s().addGroup("region", { x: 0, y: 0 });
    const a = s().addNode(REGISTRY, "app", "go", { x: 50, y: 50 });
    const b = s().addNode(REGISTRY, "sql", "postgresql", { x: 900, y: 50 });
    s().addEdge(a, b, "query");
    expect(s().doc.nodes.find((n) => n.id === a)!.group_id).toBe(zone);
    s().removeItems([b]);
    expect(s().doc.edges).toEqual([]);
    s().removeItems([zone]);
    expect(s().doc.nodes.find((n) => n.id === a)!.group_id).toBeNull();
  });
});

describe("undo grouping", () => {
  it("each edit is one step; undo and redo walk them", () => {
    const id = s().addNode(REGISTRY, "cache", "redis", { x: 0, y: 0 });
    s().updateNode(id, { label: "Hot keys" });
    expect(past()).toBe(2);
    undo();
    expect(s().doc.nodes[0].label).toBe("");
    undo();
    expect(s().doc.nodes).toEqual([]);
    redo();
    redo();
    expect(s().doc.nodes[0].label).toBe("Hot keys");
  });

  it("a whole drag is one undo step", () => {
    const id = s().addNode(REGISTRY, "app", "go", { x: 0, y: 0 });
    const before = past();
    s().beginDrag();
    for (let i = 1; i <= 20; i++) s().moveItems([{ id, position: { x: i * 10, y: i * 5 } }]);
    s().endDrag();
    expect(past()).toBe(before + 1);
    expect(s().doc.nodes[0].position).toEqual({ x: 200, y: 100 });
    undo();
    expect(s().doc.nodes[0].position).toEqual({ x: 0, y: 0 });
  });

  it("a click without movement records nothing", () => {
    s().addNode(REGISTRY, "app", "go", { x: 0, y: 0 });
    const before = past();
    s().beginDrag();
    s().endDrag();
    expect(past()).toBe(before);
  });

  it("moving a zone moves its members, nested zones included, as one undo step", () => {
    const region = s().addGroup("region", { x: 0, y: 0 }); // 640 × 420
    const az = s().addGroup("az", { x: 40, y: 40 }); // 360 × 260, inside the region
    const inAz = s().addNode(REGISTRY, "app", "go", { x: 60, y: 80 });
    const inRegion = s().addNode(REGISTRY, "cache", "redis", { x: 450, y: 300 });
    const outside = s().addNode(REGISTRY, "client", "web", { x: 900, y: 0 });
    const byId = (id: string) => [...s().doc.nodes, ...s().doc.groups].find((x) => x.id === id)!;
    expect(byId(az)).toMatchObject({ parent_id: region });
    expect(byId(inAz)).toMatchObject({ group_id: az });
    expect(byId(inRegion)).toMatchObject({ group_id: region });
    expect(byId(outside)).toMatchObject({ group_id: null });

    const before = past();
    s().beginDrag();
    s().moveItems([{ id: region, position: { x: 50, y: 10 } }]);
    s().moveItems([{ id: region, position: { x: 100, y: 20 } }]);
    s().endDrag();
    expect(past()).toBe(before + 1);
    expect(byId(region).position).toEqual({ x: 100, y: 20 });
    expect(byId(az).position).toEqual({ x: 140, y: 60 });
    expect(byId(inAz).position).toEqual({ x: 160, y: 100 });
    expect(byId(inRegion).position).toEqual({ x: 550, y: 320 });
    expect(byId(outside).position).toEqual({ x: 900, y: 0 });

    undo();
    expect(byId(region).position).toEqual({ x: 0, y: 0 });
    expect(byId(az).position).toEqual({ x: 40, y: 40 });
    expect(byId(inAz).position).toEqual({ x: 60, y: 80 });
    expect(byId(inRegion).position).toEqual({ x: 450, y: 300 });
  });

  it("a card dragged out of a zone leaves it when the drag ends", () => {
    s().addGroup("az", { x: 0, y: 0 });
    const id = s().addNode(REGISTRY, "app", "go", { x: 20, y: 20 });
    s().beginDrag();
    s().moveItems([{ id, position: { x: 800, y: 600 } }]);
    s().endDrag();
    expect(s().doc.nodes[0].group_id).toBeNull();
    undo();
    expect(s().doc.nodes[0].group_id).not.toBeNull();
  });
});

describe("zone membership", () => {
  it("a card joins the innermost zone holding its centre; an AZ joins its region", () => {
    s().addGroup("region", { x: 0, y: 0 });
    const az = s().addGroup("az", { x: 0, y: 0 });
    const id = s().addNode(REGISTRY, "app", "go", { x: 10, y: 10 });
    const doc = withMembership(s().doc);
    expect(doc.nodes.find((n) => n.id === id)!.group_id).toBe(az);
  });
});

describe("icons", () => {
  it("every registry icon has a glyph, and every kind's glyph is distinct", () => {
    const icons = [...REGISTRY.kinds.map((k) => k.icon), ...REGISTRY.meta.map((m) => m.icon)];
    expect(icons.filter((i) => !GLYPHS[i])).toEqual([]);
    const kindGlyphs = REGISTRY.kinds.map((k) => GLYPHS[k.icon]);
    expect(new Set(kindGlyphs).size).toBe(25);
  });
});

describe("autoscaling wrappers (F2)", () => {
  const policy = { flavour: "kubernetes_hpa" as const, props: { min_instances: 2, max_instances: 8 } };

  it("compute cards join the wrapper; other cards join the zone around it", () => {
    const az = s().addGroup("az", { x: 0, y: 0 }); // 360 × 260
    const asg = s().addGroup("autoscale", { x: 20, y: 20 }, policy); // 300 × 160
    const app = s().addNode(REGISTRY, "app", "go", { x: 40, y: 40 });
    const db = s().addNode(REGISTRY, "sql", "postgresql", { x: 40, y: 60 });
    const g = s().doc.groups.find((x) => x.id === asg)!;
    expect(g.parent_id).toBe(az);
    expect(g.scaling).toEqual(policy);
    expect(s().doc.nodes.find((n) => n.id === app)!.group_id).toBe(asg);
    expect(s().doc.nodes.find((n) => n.id === db)!.group_id).toBe(az);
    // Wrappers draw after zones.
    expect(s().doc.groups.map((x) => x.kind)).toEqual(["az", "autoscale"]);
  });

  it("only wrappers carry a policy, and it can be edited", () => {
    const region = s().addGroup("region", { x: 0, y: 0 }, policy);
    expect(s().doc.groups.find((x) => x.id === region)!.scaling).toBeUndefined();
    const asg = s().addGroup("autoscale", { x: 900, y: 0 }, policy);
    s().updateScaling(asg, { ...policy, props: { ...policy.props, max_instances: 20 } });
    expect(s().doc.groups.find((x) => x.id === asg)!.scaling!.props.max_instances).toBe(20);
  });
});
