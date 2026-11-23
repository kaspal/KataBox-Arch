// Animated export (M7b.2): Rust steps the run headless and returns its frames
// (`ka_app::frames::pack`); the UI draws each one on an offscreen canvas from the design's
// static picture plus `frameOps`, and records the canvas as WebM.
//
// - `MediaRecorder` stamps frames by the wall clock, so frames are painted in real time:
//   each frame is scheduled from the start time (`start + i / fps`), not chained after the
//   last, so lateness never adds up over a long clip (review B5). A 60 s clip takes ~60 s.
// - The recording stops if the window is hidden (timers are throttled there), and on Cancel.
import type { Design } from "@/bindings";
import { decodeFrame, type Frame } from "@/solve/run/frames";
import { newRing } from "@/solve/run/PacketLayer";
import { HUD_H, frameOps, paintOps, prepareRing, sceneLayout, type FrameWords, type Palette, type SceneLayout } from "@/solve/export/scene";

/** WebM is capped at 60 s, GIF at 20 s (review A7). */
export const WEBM_MAX_S = 60;
export const GIF_MAX_S = 20;
/** The video's size: at most 1280 × 720 (the scene scaled to fit), plus the HUD band. */
export const MAX_W = 1280;
export const MAX_H = 720;
export const MIN_W = 640;

export type FramesInfo = {
  duration_s: number;
  sample_factor: number;
  nodes: string[];
  edges: string[];
  fps: number;
  from_s: number;
  design: Design;
};

export class AnimationError extends Error {}

/** Unpack `u32 info_len | info JSON | u32 n | (u32 len | frame)*` (little-endian). */
export function unpackFrames(buf: ArrayBuffer): { info: FramesInfo; frames: Frame[] } {
  const v = new DataView(buf);
  if (buf.byteLength < 8) throw new AnimationError("frames: too short");
  const infoLen = v.getUint32(0, true);
  let at = 4 + infoLen;
  if (at + 4 > buf.byteLength) throw new AnimationError("frames: bad info length");
  const info = JSON.parse(new TextDecoder().decode(new Uint8Array(buf, 4, infoLen))) as FramesInfo;
  const n = v.getUint32(at, true);
  at += 4;
  const frames: Frame[] = [];
  for (let i = 0; i < n; i++) {
    if (at + 4 > buf.byteLength) throw new AnimationError("frames: truncated");
    const len = v.getUint32(at, true);
    at += 4;
    if (at + len > buf.byteLength) throw new AnimationError("frames: truncated");
    frames.push(decodeFrame(buf.slice(at, at + len)));
    at += len;
  }
  if (at !== buf.byteLength) throw new AnimationError("frames: trailing bytes");
  return { info, frames };
}

/** The WebM type this web view can record, or null (WebKitGTK has no MediaRecorder). */
export function webmType(): string | null {
  if (typeof MediaRecorder === "undefined" || typeof HTMLCanvasElement === "undefined") return null;
  if (typeof HTMLCanvasElement.prototype.captureStream !== "function") return null;
  for (const t of ["video/webm;codecs=vp9", "video/webm;codecs=vp8", "video/webm"]) {
    if (MediaRecorder.isTypeSupported(t)) return t;
  }
  return null;
}

/** Pixel size and scale for a scene: fit inside MAX_W × MAX_H, even sizes, HUD below. */
export function videoSize(layout: SceneLayout): { w: number; h: number; scale: number; offsetX: number } {
  const { w, h } = layout.bounds;
  const scale = Math.min(MAX_W / w, (MAX_H - HUD_H) / h, 2);
  const even = (n: number) => Math.max(2, Math.round(n / 2) * 2);
  // A tall, thin design is centred in at least MIN_W, so the HUD line fits and the video
  // isn't a sliver.
  const width = even(Math.max(MIN_W, w * scale));
  return { w: width, h: even(h * scale + HUD_H), scale, offsetX: Math.round((width - w * scale) / 2) };
}

export type Clock = { now: () => number; sleep: (ms: number) => Promise<void> };

export const realClock: Clock = {
  now: () => performance.now(),
  sleep: (ms) => new Promise((r) => setTimeout(r, Math.max(0, ms))),
};

/**
 * Call `paint(i)` for every frame at `start + i × 1000 / fps` on `clock` (B5: scheduled from
 * the start, so lateness doesn't add up). Stops early when `stop()` says so; returns the
 * number of frames painted.
 */
export async function paced(n: number, fps: number, paint: (i: number) => void, clock: Clock, stop: () => boolean): Promise<number> {
  const start = clock.now();
  let i = 0;
  for (; i < n; i++) {
    if (stop()) break;
    await clock.sleep(start + (i * 1000) / fps - clock.now());
    if (stop()) break;
    paint(i);
  }
  return i;
}

/** Everything one frame's picture needs, built once per export. */
export type Renderer = { canvas: HTMLCanvasElement; paint: (i: number) => void };

export function renderer(
  frames: readonly Frame[],
  info: FramesInfo,
  background: CanvasImageSource,
  palette: Palette,
  words: FrameWords,
): Renderer {
  const layout = sceneLayout(info.design, info.nodes, info.edges);
  const { w, h, scale, offsetX } = videoSize(layout);
  const canvas = document.createElement("canvas");
  canvas.width = w;
  canvas.height = h;
  const ctx = canvas.getContext("2d");
  if (!ctx) throw new AnimationError("no 2D canvas");
  const ring = newRing();
  prepareRing(ring, layout);
  const frameMs = 1000 / info.fps;
  return {
    canvas,
    paint: (i) => {
      const f = frames[i];
      if (!f) return;
      ctx.setTransform(1, 0, 0, 1, 0, 0);
      ctx.fillStyle = palette.canvas;
      ctx.fillRect(0, 0, w, h);
      ctx.drawImage(background, offsetX, 0, layout.bounds.w * scale, layout.bounds.h * scale);
      paintOps(ctx, frameOps(layout, f, ring, i * frameMs, frameMs, palette, words), layout, scale, palette, offsetX);
    },
  };
}

/** Record the frames as WebM bytes, in real time. `onProgress(i, n)` after each frame. */
export async function recordWebm(
  r: Renderer,
  n: number,
  fps: number,
  opts: { type: string; clock?: Clock; stop: () => boolean; onProgress?: (i: number, n: number) => void },
): Promise<Uint8Array | null> {
  const stream = r.canvas.captureStream(0);
  const track = stream.getVideoTracks()[0] as CanvasCaptureMediaStreamTrack | undefined;
  const recorder = new MediaRecorder(stream, { mimeType: opts.type, videoBitsPerSecond: 4_000_000 });
  const chunks: Blob[] = [];
  recorder.ondataavailable = (e) => {
    if (e.data.size > 0) chunks.push(e.data);
  };
  const done = new Promise<void>((resolve) => {
    recorder.onstop = () => resolve();
  });
  recorder.start(1000);
  const painted = await paced(
    n,
    fps,
    (i) => {
      r.paint(i);
      track?.requestFrame();
      opts.onProgress?.(i + 1, n);
    },
    opts.clock ?? realClock,
    opts.stop,
  );
  recorder.stop();
  await done;
  for (const t of stream.getTracks()) t.stop();
  if (painted < n) return null;
  return new Uint8Array(await new Blob(chunks, { type: "video/webm" }).arrayBuffer());
}
