# ADR 0004: The free-form canvas and edge routing

- Status: accepted (M0, 2026-11-21). This ADR is the source of truth for the code until the spec is synced.

## Context

The approved mockups show cards placed anywhere, joined by smooth curves that follow when a card moves, with a protocol chip on each curve. Orthogonal routing reads as a flowchart and fights free placement.

## Decision

**Canvas.** React Flow 12, controlled by a Zustand store with zundo undo.

- **Undo.** The history holds only the design, never the selection or the viewport. One drag is one undo step: one card, a selection, a zone with all its members, or a resize. History pauses while dragging and records the whole move at the end.
- **Placement.** Snapping to an 8 px grid is on by default (`canvas.snap`); holding Alt places freely.
- **Zones.** Zones are drawn behind the cards. Only their label is a drag handle, and moving a zone moves its members. On drop, a card joins the innermost zone holding its centre, and an AZ joins the region holding its centre.
- **Window.** The Tauri window sets `dragDropEnabled: false`. Otherwise Windows' file-drop handler swallows the HTML5 drag events, and dragging from the library would do nothing. Dropping a `.kbarch` file on the window to open it is in the backlog.

**Edge routing** (`solve/geometry/routeEdge.ts`, pure and unit-tested):

- **Sides.** With centre offsets dx, dy and the two cards' mean size w × h, the edge runs horizontally when `|dx|/(w+40) ≥ |dy|/(h+40)`, otherwise vertically. It leaves the source's side facing the target and enters the target's side facing the source, at the side midpoints.
- **Curve.** A cubic Bézier whose control points extend along each side's outward normal by `k = max(36, 0.45 × gap)`, where gap is the distance between the anchors along that axis.
- **Live.** Every edge is recomputed from both cards' live boxes on each render, so it follows a drag, and its anchors switch sides as a card passes its neighbour.
- **Drawing.** The protocol chip sits at t = 0.5 and the arrowhead at the target. Publish and Consume are dashed; Replicate is dotted.
- **M1.** The packet layer samples the same curve (`bezierPoint`).

**Wiring.**

- **Where the rules live.** The rules are a table in `ka-core/src/wiring.rs`: `{protocol, from: KindSet, to: KindSet, same_kind}` plus ordered denials that give the reason code. Rust and TypeScript each run a ~15-line evaluator over the same table. A parity test checks that the TypeScript evaluator reproduces Rust's exported 25 × 25 matrix exactly (252 allowed pairs today), so the logic is not duplicated in a way that can drift.
- **Card-level checks.** Self-loops and protocols already used for a pair.
- **Table highlights.**
  - Clients never reach a data store directly. The one exception is `client → object` over HTTP (presigned uploads).
  - Consume edges start at a queue, stream, pub/sub or DLQ and end at a service.
  - `cdn → object` (origin); `sql`/`nosql → stream` publish (CDC, outbox); `scheduler →` services over HTTP; `stream → stream` replicate (mirroring); `pubsub → queue` publish (fan-out).
  - DNS is an ordinary edge hop in M0; M1 models it as resolution.

**Keyboard wiring.** Select a card and press C. The arrow keys pick the nearest card in that direction (within a 2:1 cone); Tab and Shift+Tab cycle in reading order. Enter connects, opening the protocol picker when there is a choice; Escape cancels. A live region announces each target and any block reason.

## Consequences

Edge geometry has one implementation, shared by the editor now and the packet animation later. Changing a wiring rule is a data edit in Rust, a reviewed snapshot update (`KA_UPDATE_SNAPSHOTS=1`) and a fixture regeneration. The UI follows automatically.
