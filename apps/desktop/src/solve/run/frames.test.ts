import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";
import { decodeFrame, FrameError, HEADER_BYTES, NODE_BYTES, PACKET_ASYNC, PACKET_BYTES, PACKET_HIT } from "@/solve/run/frames";

// A frame Rust encoded (ka_app::live::fixture_frame): client → app → cache + Postgres at
// 2,000 rps, three simulated seconds in. Regenerated with the export-bindings bin.
function fixture(): ArrayBuffer {
  const bytes = readFileSync(resolve(__dirname, "../../test/fixtures/frame.kaf"));
  return bytes.buffer.slice(bytes.byteOffset, bytes.byteOffset + bytes.byteLength);
}

describe("live-run frames", () => {
  it("decode a frame Rust wrote", () => {
    const f = decodeFrame(fixture());
    expect(f.frame).toBe(90);
    expect(f.tMs).toBe(2999);
    expect(f.windowMs).toBe(33);
    expect(f.playing).toBe(true);
    expect(f.finished).toBe(false);
    expect(f.speed).toBe(1);
    expect(f.rps).toBeGreaterThan(1500);
    expect(f.rps).toBeLessThan(2500);
    expect(f.p95Ms).toBeGreaterThan(0);
    expect(f.errorRate).toBe(0);
    expect(f.costMonth).toBeGreaterThan(0);
    // Sandbox writes spread over a million keys: few cached copies predate a write yet.
    expect(f.staleRate).toBeGreaterThanOrEqual(0);
    expect(f.staleRate).toBeLessThan(0.05);
    expect(f.util).toHaveLength(4);
    expect(f.util[0]).toBe(0); // the client
    expect(f.util[1]).toBeGreaterThan(0); // the app
    expect(f.packetCount).toBeGreaterThan(0);
    expect(f.packetTotal).toBeGreaterThanOrEqual(f.packetCount);
    let hits = 0;
    for (let i = 0; i < f.packetCount; i++) {
      expect(f.packetEdge[i]).toBeLessThan(3);
      expect(f.packetType[i] & 7).toBeLessThan(2); // read or write
      expect(f.packetType[i] & PACKET_ASYNC).toBe(0); // no brokers in this design
      expect(f.packetStart[i]).toBeLessThanOrEqual(34);
      if (f.packetType[i] & PACKET_HIT) {
        hits++;
        expect(f.packetEdge[i]).toBe(1); // app → cache
        expect(f.packetTravel[i]).toBe(0);
      }
    }
    expect(hits).toBeGreaterThan(0);
  });

  it("refuse anything that isn't a whole KAF1 frame", () => {
    const good = fixture();
    expect(() => decodeFrame(good.slice(0, 10))).toThrow(FrameError);
    expect(() => decodeFrame(good.slice(0, good.byteLength - 1))).toThrow(FrameError);
    const bad = good.slice(0);
    new Uint8Array(bad)[0] = 0x58;
    expect(() => decodeFrame(bad)).toThrow(FrameError);
  });

  it("place node records by their index", () => {
    const buf = new ArrayBuffer(HEADER_BYTES + 2 * NODE_BYTES + PACKET_BYTES);
    const v = new DataView(buf);
    [0x4b, 0x41, 0x46, 0x31].forEach((b, i) => v.setUint8(i, b));
    v.setUint8(4, 1);
    v.setUint8(5, 2); // finished
    v.setUint16(36, 2, true);
    v.setUint16(38, 1, true);
    v.setUint32(40, 9, true);
    v.setFloat32(44, 0.25, true);
    v.setFloat32(48, 1234, true);
    // Records in reverse order: node 1, then node 0.
    v.setUint16(52, 1, true);
    v.setUint8(54, 97);
    v.setUint16(55, 300, true);
    v.setUint8(57, 3);
    v.setUint16(58, 0, true);
    v.setUint8(60, 5);
    v.setUint16(64, 1, true);
    v.setUint8(66, 1 | PACKET_ASYNC);
    v.setUint16(67, 12, true);
    v.setUint16(69, 7, true);
    const f = decodeFrame(buf);
    expect(f.finished).toBe(true);
    expect(f.packetTotal).toBe(9);
    expect(f.staleRate).toBe(0.25);
    expect(f.retried).toBe(1234);
    expect(f.playing).toBe(false);
    expect([...f.util]).toEqual([5, 97]);
    expect([...f.queue]).toEqual([0, 300]);
    expect([...f.nodeFlags]).toEqual([0, 3]);
    expect([f.packetEdge[0], f.packetType[0], f.packetStart[0], f.packetTravel[0]]).toEqual([1, 9, 12, 7]);
  });
});
