import { describe, expect, it } from "vitest";
import { routeEdge, slotPosition, type Rect } from "@/solve/geometry/routeEdge";
import { assignSlots } from "@/solve/geometry/slots";

const card = (x: number, y: number): Rect => ({ x, y, w: 200, h: 60 });

describe("edge slots (F1)", () => {
  it("a single edge on a side stays centred", () => {
    const rects = new Map([
      ["a", card(0, 0)],
      ["b", card(400, 0)],
    ]);
    const s = assignSlots([{ id: "e1", from: "a", to: "b" }], rects);
    expect(s.get("e1")).toEqual({ source: { slot: 0, count: 1 }, target: { slot: 0, count: 1 } });
    expect(slotPosition(s.get("e1")!.source)).toBe(0.5);
  });

  it("ends sharing a side are ordered by the far end and spread within the middle 70%", () => {
    // Three cards below "hub", left to right: edges from them all enter hub's bottom side.
    const rects = new Map([
      ["hub", card(300, 0)],
      ["l", card(0, 300)],
      ["m", card(300, 300)],
      ["r", card(600, 300)],
    ]);
    const edges = [
      { id: "e-r", from: "r", to: "hub" },
      { id: "e-l", from: "l", to: "hub" },
      { id: "e-m", from: "m", to: "hub" },
    ];
    const s = assignSlots(edges, rects);
    expect([s.get("e-l")!.target, s.get("e-m")!.target, s.get("e-r")!.target]).toEqual([
      { slot: 0, count: 3 },
      { slot: 1, count: 3 },
      { slot: 2, count: 3 },
    ]);
    const xs = ["e-l", "e-m", "e-r"].map((id) => routeEdge(rects.get(id.slice(2))!, rects.get("hub")!, s.get(id)).p3.x);
    // Evenly spaced inside [15%, 85%] of hub's width, left to right: no crossing.
    expect(xs[0]).toBeCloseTo(300 + 200 * (0.15 + 0.7 / 6));
    expect(xs[1]).toBeCloseTo(400);
    expect(xs[2]).toBeCloseTo(300 + 200 * (0.15 + (0.7 * 5) / 6));
    expect(xs[0] < xs[1] && xs[1] < xs[2]).toBe(true);
  });

  it("the order follows a card as it moves past its neighbour", () => {
    const at = (x: number) => {
      const rects = new Map([
        ["hub", card(300, 0)],
        ["a", card(100, 300)],
        ["b", card(x, 300)],
      ]);
      const s = assignSlots(
        [
          { id: "ea", from: "a", to: "hub" },
          { id: "eb", from: "b", to: "hub" },
        ],
        rects,
      );
      return s.get("eb")!.target.slot;
    };
    expect(at(500)).toBe(1); // b right of a
    expect(at(-200)).toBe(0); // b moved past a, to its left
  });

  it("source and target ends of one edge get their own slots", () => {
    const rects = new Map([
      ["a", card(0, 0)],
      ["b", card(400, 0)],
      ["c", card(400, 30)],
    ]);
    const s = assignSlots(
      [
        { id: "ab", from: "a", to: "b" },
        { id: "ac", from: "a", to: "c" },
      ],
      rects,
    );
    expect(s.get("ab")!.source).toEqual({ slot: 0, count: 2 });
    expect(s.get("ac")!.source).toEqual({ slot: 1, count: 2 });
    expect(s.get("ab")!.target).toEqual({ slot: 0, count: 1 });
  });
});
