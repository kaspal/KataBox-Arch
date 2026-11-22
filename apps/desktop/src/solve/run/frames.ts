// Decodes the binary frames of a live run (`KAF1`, little-endian). The layout is defined
// once, in crates/ka-app/src/live.rs; `frames.test.ts` decodes a frame Rust wrote.

export const HEADER_BYTES = 40;
export const NODE_BYTES = 6;
export const PACKET_BYTES = 7;

const FLAG_PLAYING = 1;
const FLAG_FINISHED = 2;
export const NODE_DOWN = 1;
export const NODE_OVERLOADED = 2;
export const PACKET_ASYNC = 8;

export type Frame = {
  frame: number;
  /** Simulated time, ms. */
  tMs: number;
  /** Simulated ms since the previous frame. */
  windowMs: number;
  playing: boolean;
  finished: boolean;
  speed: number;
  /** The last completed simulated second. */
  p95Ms: number;
  rps: number;
  errorRate: number;
  /** Monthly cost at the traffic seen so far. */
  costMonth: number;
  /** By node index: utilisation (0–100), queued requests, flags (NODE_DOWN, NODE_OVERLOADED). */
  util: Uint8Array;
  queue: Uint16Array;
  nodeFlags: Uint8Array;
  /** Packets: edge index, request type (bits 0–2) | PACKET_ASYNC, start (ms into the window), travel (0.1 ms). */
  packetCount: number;
  packetEdge: Uint16Array;
  packetType: Uint8Array;
  packetStart: Uint16Array;
  packetTravel: Uint16Array;
};

export class FrameError extends Error {}

export function decodeFrame(buf: ArrayBuffer): Frame {
  const v = new DataView(buf);
  if (buf.byteLength < HEADER_BYTES) throw new FrameError("short frame");
  const magic = String.fromCharCode(v.getUint8(0), v.getUint8(1), v.getUint8(2), v.getUint8(3));
  if (magic !== "KAF1" || v.getUint8(4) !== 1) throw new FrameError("not a KAF1 frame");
  const flags = v.getUint8(5);
  const nodeCount = v.getUint16(36, true);
  const packetCount = v.getUint16(38, true);
  if (buf.byteLength !== HEADER_BYTES + nodeCount * NODE_BYTES + packetCount * PACKET_BYTES) {
    throw new FrameError("frame length does not match its counts");
  }
  const util = new Uint8Array(nodeCount);
  const queue = new Uint16Array(nodeCount);
  const nodeFlags = new Uint8Array(nodeCount);
  let at = HEADER_BYTES;
  for (let i = 0; i < nodeCount; i++, at += NODE_BYTES) {
    const idx = v.getUint16(at, true);
    if (idx >= nodeCount) throw new FrameError("node index out of range");
    util[idx] = v.getUint8(at + 2);
    queue[idx] = v.getUint16(at + 3, true);
    nodeFlags[idx] = v.getUint8(at + 5);
  }
  const packetEdge = new Uint16Array(packetCount);
  const packetType = new Uint8Array(packetCount);
  const packetStart = new Uint16Array(packetCount);
  const packetTravel = new Uint16Array(packetCount);
  for (let i = 0; i < packetCount; i++, at += PACKET_BYTES) {
    packetEdge[i] = v.getUint16(at, true);
    packetType[i] = v.getUint8(at + 2);
    packetStart[i] = v.getUint16(at + 3, true);
    packetTravel[i] = v.getUint16(at + 5, true);
  }
  return {
    frame: v.getUint32(6, true),
    tMs: v.getUint32(10, true),
    windowMs: v.getUint16(14, true),
    playing: (flags & FLAG_PLAYING) !== 0,
    finished: (flags & FLAG_FINISHED) !== 0,
    speed: v.getFloat32(16, true),
    p95Ms: v.getFloat32(20, true),
    rps: v.getFloat32(24, true),
    errorRate: v.getFloat32(28, true),
    costMonth: v.getFloat32(32, true),
    util,
    queue,
    nodeFlags,
    packetCount,
    packetEdge,
    packetType,
    packetStart,
    packetTravel,
  };
}
