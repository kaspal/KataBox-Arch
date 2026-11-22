import { describe, expect, it } from "vitest";
import { chipFits, chipWidth } from "@/solve/edges/CurvedEdge";

describe("protocol chips on short connections (review F5)", () => {
  it("fit only with 8 px to spare on each side", () => {
    const w = chipWidth("Replicate");
    expect(chipFits(w + 16, "Replicate")).toBe(true);
    expect(chipFits(w + 15, "Replicate")).toBe(false);
    // Primary <-> Replica side by side in flaw #9: about 52 px between the cards.
    expect(chipFits(52, "Replicate")).toBe(false);
    expect(chipFits(170, "HTTP")).toBe(true);
  });
});
