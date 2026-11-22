import { describe, expect, it } from "vitest";
import { cycle, nearestInDirection, type Placed } from "@/solve/keyboardWiring";

//        n
//   w    s    e
//        b         far-e (off axis)
const cards: Placed[] = [
  { id: "s", x: 0, y: 0 },
  { id: "n", x: 0, y: -100 },
  { id: "e", x: 200, y: 10 },
  { id: "w", x: -200, y: 0 },
  { id: "b", x: 10, y: 150 },
  { id: "far-e", x: 150, y: 140 },
];
const s = cards[0];

describe("keyboard wiring targets", () => {
  it("picks the nearest card in each direction, preferring cards straight ahead", () => {
    expect(nearestInDirection(cards, s, "up")).toBe("n");
    expect(nearestInDirection(cards, s, "right")).toBe("e");
    expect(nearestInDirection(cards, s, "left")).toBe("w");
    expect(nearestInDirection(cards, s, "down")).toBe("b");
  });

  it("returns null when nothing lies that way, and skips excluded cards", () => {
    expect(nearestInDirection(cards, cards[3], "left")).toBeNull();
    expect(nearestInDirection(cards, s, "right", ["e"])).toBe("far-e");
    // b is below, not to the right: outside the cone.
    expect(nearestInDirection(cards, s, "right", ["e", "far-e"])).toBeNull();
  });

  it("Tab cycles in reading order, wrapping around", () => {
    expect(cycle(cards, null, 1, ["s"])).toBe("n");
    expect(cycle(cards, "n", 1, ["s"])).toBe("w");
    expect(cycle(cards, "w", 1, ["s"])).toBe("e");
    expect(cycle(cards, "e", 1, ["s"])).toBe("b");
    expect(cycle(cards, "b", 1, ["s"])).toBe("far-e");
    expect(cycle(cards, "far-e", 1, ["s"])).toBe("n");
    expect(cycle(cards, "n", -1, ["s"])).toBe("far-e");
  });
});
