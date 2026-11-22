import { describe, expect, it } from "vitest";
import {
  MIN_K,
  bezierAngle,
  bezierPoint,
  facingSides,
  routeEdge,
  sideMidpoint,
  toPath,
  type Rect,
} from "@/solve/geometry/routeEdge";

const card = (x: number, y: number, w = 200, h = 60): Rect => ({ x, y, w, h });
const A = card(0, 0);

describe("routeEdge", () => {
  it("uses horizontal sides when the target is mostly to the side", () => {
    const r = routeEdge(A, card(400, 30));
    expect([r.sourceSide, r.targetSide]).toEqual(["right", "left"]);
    expect(r.p0).toEqual({ x: 200, y: 30 });
    expect(r.p3).toEqual({ x: 400, y: 60 });
    const back = routeEdge(A, card(-400, 0));
    expect([back.sourceSide, back.targetSide]).toEqual(["left", "right"]);
  });

  it("uses vertical sides when the target is mostly above or below", () => {
    const r = routeEdge(A, card(40, 300));
    expect([r.sourceSide, r.targetSide]).toEqual(["bottom", "top"]);
    expect(r.p0).toEqual({ x: 100, y: 60 });
    expect(r.p3).toEqual({ x: 140, y: 300 });
    const up = routeEdge(A, card(0, -300));
    expect([up.sourceSide, up.targetSide]).toEqual(["top", "bottom"]);
  });

  it("switches sides exactly at |dx|/(w+40) = |dy|/(h+40)", () => {
    // Centre offsets: w + 40 = 240, h + 40 = 100. dy = 100 → threshold dx = 240.
    const at = (dx: number) => facingSides(A, card(dx, 100));
    expect(at(240).source).toBe("right"); // equal ratios: horizontal wins
    expect(at(239).source).toBe("bottom");
    expect(at(-240).source).toBe("left");
  });

  it("anchors switch sides as a card moves past its neighbour", () => {
    // Move the target from far left, under, to far right of the source.
    const sides = [-600, -300, 0, 300, 600].map((x) => routeEdge(A, card(x, 120)).sourceSide);
    expect(sides).toEqual(["left", "left", "bottom", "right", "right"]);
  });

  it("anchors are the facing sides' midpoints", () => {
    const b = card(500, 0, 120, 80);
    const r = routeEdge(A, b);
    expect(r.p0).toEqual(sideMidpoint(A, "right"));
    expect(r.p3).toEqual(sideMidpoint(b, "left"));
  });

  it("control points extend along each side's normal by max(36, 0.45 × gap)", () => {
    const far = routeEdge(A, card(600, 0));
    const gap = 600 - 200;
    expect(far.p1).toEqual({ x: 200 + 0.45 * gap, y: 30 });
    expect(far.p2).toEqual({ x: 600 - 0.45 * gap, y: 30 });
    const near = routeEdge(A, card(250, 0));
    expect(near.p1).toEqual({ x: 200 + MIN_K, y: 30 });
    expect(near.p2).toEqual({ x: 250 - MIN_K, y: 30 });
    const below = routeEdge(A, card(0, 400));
    expect(below.p1).toEqual({ x: 100, y: 60 + 0.45 * 340 });
  });

  it("is symmetric: swapping the cards reverses the curve", () => {
    const b = card(380, 170, 160, 70);
    const ab = routeEdge(A, b);
    const ba = routeEdge(b, A);
    expect(ba.p0).toEqual(ab.p3);
    expect(ba.p3).toEqual(ab.p0);
    expect(ba.p1).toEqual(ab.p2);
    expect(ba.p2).toEqual(ab.p1);
  });

  it("the midpoint (t = 0.5) lies on the curve, between the anchors", () => {
    const r = routeEdge(A, card(500, 200));
    const mid = bezierPoint(r, 0.5);
    expect(mid.x).toBeCloseTo((r.p0.x + 3 * r.p1.x + 3 * r.p2.x + r.p3.x) / 8);
    expect(mid.y).toBeCloseTo((r.p0.y + 3 * r.p1.y + 3 * r.p2.y + r.p3.y) / 8);
    expect(bezierPoint(r, 0)).toEqual(r.p0);
    expect(bezierPoint(r, 1)).toEqual(r.p3);
    // A straight horizontal edge: the chip sits halfway.
    const straight = routeEdge(A, card(600, 0));
    expect(bezierPoint(straight, 0.5)).toEqual({ x: 400, y: 30 });
  });

  it("the arrow points into the target", () => {
    expect(bezierAngle(routeEdge(A, card(600, 0)), 1)).toBeCloseTo(0); // →
    expect(bezierAngle(routeEdge(A, card(0, 400)), 1)).toBeCloseTo(Math.PI / 2); // ↓
  });

  it("renders an SVG cubic path", () => {
    expect(toPath(routeEdge(A, card(600, 0)))).toBe("M200,30 C380,30 420,30 600,30");
  });
});
