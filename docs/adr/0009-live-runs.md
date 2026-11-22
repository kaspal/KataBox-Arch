# ADR 0009: Live runs and the frame stream

- Status: accepted (M1.3, 2026-11-22)

## Context

A design should come alive when the user presses Run: requests moving, cards filling up, numbers ticking. The
simulator (ADR 0006) is deterministic and much faster than real time. The UI needs a paced, controllable view of it:
pause, speed, seek. And it needs that without turning thousands of packets a second into JSON.

## Decision

### Where it runs
- **`ka-app/src/live.rs`:**
  - `LiveCore` is the deterministic part: step, snapshot every 5 simulated seconds, seek, encode a frame. It has no
    clock and no thread, so tests drive it directly.
  - `LiveRun` puts a `LiveCore` on a worker thread and paces it to wall time × speed (0.25–8×), at about 30 frames
    a second.
  - Wall time is used only here, never in `ka-sim`.
- **One run at a time.** `App` keeps the current run by id; starting a new one stops the old.
- **Commands:**
  - `start_run(design, preset, speed, on_frame)`;
  - `control_run(id, pause | resume | seek | speed | stop)`;
  - `run_result(id)`.
- **The simulator side.** `ka-sim` gained read-only views:
  - a packet trace (forward hops), off unless a live run asks for it;
  - `Engine::snapshot()`: busy server-time, queues and down flags per card, plus the last completed second.

  Tests prove that tracing and stepping in frame-sized chunks leave the result byte-identical to one run.

### Seeking
- `seek(t)` restores the nearest snapshot at or before `t`, then re-simulates to `t`. The engine is `Clone`.
- The state is exactly the one playing through reaches. A test compares both snapshots and both final results.

### Frames (`KAF1`)
- **The format:** packed little-endian binary, specified in `live.rs` and decoded in `solve/run/frames.ts`:
  - a 40-byte header (time, flags, speed, four tiles);
  - 6 bytes per card (utilisation, queue, flags);
  - 7 bytes per packet (edge, request type and async bit, start offset, travel time).
- **Packets** are thinned to at most 100 per frame, with a deterministic stride.
- **Utilisation** is measured over the frame's window, from busy server-time.
- **Tiles** describe the last completed simulated second.
- **Parity:** the export-bindings bin writes a frame Rust encoded to `src/test/fixtures/frame.kaf`, and the decoder's
  test reads it.

### Transport (plan ★7)
- **The channel.** Frames travel over a Tauri `Channel` as raw IPC bodies (`InvokeResponseBody::Raw`) and arrive as
  ArrayBuffers. JSON is never used per packet.
- **The typing workaround.** tauri-specta can't type a `Channel<InvokeResponseBody>`, so the command takes a request
  struct whose `on_frame` is a `JavaScriptChannelId` typed as a string. `lib/api.ts` creates the `Channel` and
  passes it there, and stays the only door to the commands.
- **No fallback needed.** The base64 fallback in the plan wasn't needed; verified in the real app.

### Sandbox traffic (plan ★8)
- **Load:** the client cards' `rps`, split into 90% reads and 10% writes. Each type takes the most complete route the
  design supports:
  - reads: `edge? → compute → cache? → datastore`, then `… → cache`, then `edge? → compute`;
  - writes: `… → datastore:write`, then `edge? → compute`.
- **Presets:** Normal, and Spike ×4 (from 20 s to 35 s), over 60 s.
- **Kill a node** arrives with chaos in M1.5.

### Editing during a run (plan ★13)
- **What restarts a run.** An edit to anything the simulator reads restarts the run from 0, with the same seed and
  preset. The old metrics stay until the new run's first frame. What the simulator reads: kinds, flavours, props,
  edges and zone membership.
- **What doesn't.** Moving a card doesn't restart. Opening another design, or leaving the Sandbox, stops the run.

## Consequences
- **The packet layer (M1.4)** draws from the same frames, through `onFrame` listeners outside React state.
- **Timeline and chaos (M1.5)** use `seek` and `control_run`.
- **Snapshots cost memory:** 12 engine clones per 60 s run. They are bounded by the sampling cap.
