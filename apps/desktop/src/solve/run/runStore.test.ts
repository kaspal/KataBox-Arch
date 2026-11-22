import { describe, expect, it } from "vitest";
import type { Frame } from "@/solve/run/frames";
import { addToSeries, chaosAt, chaosTime } from "@/solve/run/runStore";
import { chaos, chaosKind } from "@/solve/run/chaos";

const f = (tMs: number, p95Ms = 10): Frame => ({ tMs, p95Ms, rps: 100, errorRate: 0 }) as Frame;

describe("the run series", () => {
  it("adds each completed second once, in order", () => {
    let s = addToSeries([], f(500));
    expect(s).toEqual([]);
    s = addToSeries(s, f(1_100, 12));
    s = addToSeries(s, f(1_900, 99));
    s = addToSeries(s, f(2_050, 14));
    expect(s.map((p) => [p.t, p.p95Ms])).toEqual([
      [0, 12],
      [1, 14],
    ]);
  });

  it("is cut back when the run seeks backwards, and cleared at the start", () => {
    let s = [0, 1, 2, 3, 4].reduce((acc, t) => addToSeries(acc, f((t + 1) * 1000 + 10)), [] as ReturnType<typeof addToSeries>);
    s = addToSeries(s, f(2_500));
    expect(s.map((p) => p.t)).toEqual([0, 1]);
    expect(addToSeries(s, f(200))).toEqual([]);
  });
});

describe("chaos events", () => {
  it("are built in the flattened wire shape and moved to the injection time", () => {
    const e = chaosAt(chaos.latency("db", 200, 10), 12.5);
    expect(e).toEqual({ latency: { id: "db", add_ms: 200, for: 10, at: 12.5 } });
    expect(chaosKind(e)).toBe("latency");
    expect(chaosTime(e)).toBe(12.5);
  });
});
