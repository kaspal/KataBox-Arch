// Packed export frames for tests (`ka_app::frames::pack`): header-only KAF1 frames.
import type { Design } from "@/bindings";
import { HEADER_BYTES } from "@/solve/run/frames";
import type { FramesInfo } from "@/solve/export/animation";

/** A KAF1 frame with no nodes and no packets, `i` frames in at 30 fps. */
export function emptyFrame(i: number): Uint8Array {
  const buf = new ArrayBuffer(HEADER_BYTES);
  const v = new DataView(buf);
  [0x4b, 0x41, 0x46, 0x31].forEach((b, k) => v.setUint8(k, b));
  v.setUint8(4, 1);
  v.setUint8(5, 1); // playing
  v.setUint32(6, i, true);
  v.setUint32(10, Math.round((i * 1000) / 30), true);
  v.setUint16(14, 33, true);
  v.setFloat32(16, 1, true);
  return new Uint8Array(buf);
}

/** `u32 info_len | info | u32 n | (u32 len | frame)*`. */
export function packFrames(info: FramesInfo, frames: Uint8Array[]): ArrayBuffer {
  const json = new TextEncoder().encode(JSON.stringify(info));
  const total = 8 + json.length + frames.reduce((n, f) => n + 4 + f.length, 0);
  const out = new Uint8Array(total);
  const v = new DataView(out.buffer);
  v.setUint32(0, json.length, true);
  out.set(json, 4);
  let at = 4 + json.length;
  v.setUint32(at, frames.length, true);
  at += 4;
  for (const f of frames) {
    v.setUint32(at, f.length, true);
    out.set(f, at + 4);
    at += 4 + f.length;
  }
  return out.buffer;
}

export function fakeFrames(design: Design, n: number, fromS = 0): ArrayBuffer {
  const info: FramesInfo = {
    duration_s: 60,
    sample_factor: 1,
    nodes: design.nodes.map((x) => x.id),
    edges: design.edges.map((e) => e.id),
    fps: 30,
    from_s: fromS,
    design,
  };
  return packFrames(info, Array.from({ length: n }, (_, i) => emptyFrame(i)));
}
