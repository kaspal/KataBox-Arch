import { describe, expect, it } from "vitest";
import { AnimationError, MIN_W, paced, unpackFrames, videoSize, webmType, type Clock, type FramesInfo } from "@/solve/export/animation";
import { emptyFrame, packFrames } from "@/test/fakeFrames";
import { FAKE_REFERENCE } from "@/test/fakeProblems";

const info: FramesInfo = {
  duration_s: 60,
  sample_factor: 1,
  nodes: ["ref-app", "ref-cache"],
  edges: ["ref-e1"],
  fps: 30,
  from_s: 2,
  design: FAKE_REFERENCE,
};

describe("animated export frames (M7b.2)", () => {
  it("unpack what Rust packs, and refuse anything else", () => {
    const buf = packFrames(info, [emptyFrame(0), emptyFrame(1), emptyFrame(2)]);
    const { info: got, frames } = unpackFrames(buf);
    expect(got).toEqual(info);
    expect(frames.map((f) => f.frame)).toEqual([0, 1, 2]);
    expect(() => unpackFrames(buf.slice(0, buf.byteLength - 1))).toThrow(AnimationError);
    const extra = new Uint8Array(buf.byteLength + 1);
    extra.set(new Uint8Array(buf));
    expect(() => unpackFrames(extra.buffer)).toThrow(AnimationError);
  });

  it("paint 1,800 frames for 60 s at 30 fps, each on time from the start (B5)", async () => {
    // A fake clock: sleeping moves time on, and every paint costs 5 ms. Chaining 33 ms waits
    // after each paint would drift by 9 s over the clip; scheduling from the start doesn't.
    let now = 1000;
    const clock: Clock = { now: () => now, sleep: async (ms) => void (now += Math.max(0, ms)) };
    const at: number[] = [];
    const n = await paced(1800, 30, () => {
      at.push(now - 1000);
      now += 5;
    }, clock, () => false);
    expect(n).toBe(1800);
    expect(at).toHaveLength(1800);
    at.forEach((t, i) => expect(Math.abs(t - (i * 1000) / 30)).toBeLessThan(6));
  });

  it("stop when asked", async () => {
    let now = 0;
    const clock: Clock = { now: () => now, sleep: async (ms) => void (now += Math.max(0, ms)) };
    let painted = 0;
    const n = await paced(100, 30, () => void painted++, clock, () => painted >= 10);
    expect(n).toBe(10);
  });

  it("size the video to fit 1280 x 720, centring a tall, thin design in at least 640 px", () => {
    const wide = videoSize({ bounds: { x: 0, y: 0, w: 2000, h: 600 }, cards: [], routes: [] });
    expect(wide.w).toBe(1280);
    expect(wide.offsetX).toBe(0);
    const tall = videoSize({ bounds: { x: 0, y: 0, w: 288, h: 800 }, cards: [], routes: [] });
    expect(tall.w).toBe(MIN_W);
    expect(tall.h).toBe(720);
    expect(tall.offsetX).toBeGreaterThan(150);
    expect(tall.w % 2 + tall.h % 2).toBe(0);
  });

  it("know when the web view can't record WebM", () => {
    // jsdom, like WebKitGTK, has no MediaRecorder.
    expect(webmType()).toBeNull();
  });
});
