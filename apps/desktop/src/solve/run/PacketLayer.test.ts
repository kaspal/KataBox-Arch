import { describe, expect, it } from "vitest";
import { EVENT_MS, HOP_MS, MAX_DROPS_PER_FRAME, MAX_HITS_PER_FRAME, newRing, pushFrame, RING, visible } from "@/solve/run/PacketLayer";
import { PACKET_DROP, PACKET_HIT, type Frame } from "@/solve/run/frames";

function frame(packets: { edge: number; type?: number; start: number }[], windowMs = 33): Frame {
  const n = packets.length;
  return {
    frame: 1,
    tMs: 1000,
    windowMs,
    playing: true,
    finished: false,
    speed: 1,
    p95Ms: 0,
    rps: 0,
    errorRate: 0,
    costMonth: 0,
    util: new Uint8Array(0),
    queue: new Uint16Array(0),
    nodeFlags: new Uint8Array(0),
    packetCount: n,
    packetTotal: n,
    packetEdge: Uint16Array.from(packets.map((p) => p.edge)),
    packetType: Uint8Array.from(packets.map((p) => p.type ?? 0)),
    packetStart: Uint16Array.from(packets.map((p) => p.start)),
    packetTravel: new Uint16Array(n),
  };
}

describe("the packet ring", () => {
  it("spreads a frame's packets over the frame interval and keeps each on screen for its duration", () => {
    const ring = newRing();
    pushFrame(ring, frame([{ edge: 0, start: 0 }, { edge: 1, start: 33 }, { edge: 2, start: 10, type: PACKET_DROP }]), 1000);
    expect(ring.start[0]).toBe(1000);
    expect(ring.start[1]).toBeCloseTo(1033);
    expect(ring.dur[0]).toBe(HOP_MS);
    expect(ring.dur[2]).toBe(EVENT_MS);
    expect(visible(ring, 999)).toBe(0);
    expect(visible(ring, 1040)).toBe(3);
    expect(visible(ring, 1000 + EVENT_MS + 20)).toBe(2); // the drop mark is gone
    expect(visible(ring, 1000 + HOP_MS + 40)).toBe(0);
  });

  it("keeps only a few hit and drop marks per frame", () => {
    const ring = newRing();
    const marks = [
      ...Array.from({ length: 20 }, () => ({ edge: 0, start: 1, type: PACKET_HIT })),
      ...Array.from({ length: 20 }, () => ({ edge: 1, start: 1, type: PACKET_DROP })),
      { edge: 2, start: 1 },
    ];
    pushFrame(ring, frame(marks), 0);
    expect(visible(ring, 10)).toBe(MAX_HITS_PER_FRAME + MAX_DROPS_PER_FRAME + 1);
  });

  it("ignores paused frames and wraps around without growing", () => {
    const ring = newRing();
    pushFrame(ring, frame([{ edge: 0, start: 0 }], 0), 0);
    expect(visible(ring, 1)).toBe(0);
    const many = Array.from({ length: 100 }, (_, i) => ({ edge: i % 3, start: i % 33 }));
    for (let k = 0; k < 50; k++) pushFrame(ring, frame(many), k * 33);
    expect(ring.edge).toHaveLength(RING);
    expect(ring.head).toBe((50 * 100) % RING);
    // About 600 ms of frames are on screen: ~18 frames of 100 packets, capped by the ring.
    const n = visible(ring, 49 * 33);
    expect(n).toBeGreaterThan(1500);
    expect(n).toBeLessThanOrEqual(RING);
  });
});
