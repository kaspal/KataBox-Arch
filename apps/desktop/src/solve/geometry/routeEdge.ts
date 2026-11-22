// Edge routing for the free-form canvas (ADR 0004): a smooth curve between the facing
// sides of two cards. Pure geometry, unit-tested; M1's packet layer samples the same
// curve, so this is the one place that decides where an edge goes.
//
// - Sides: horizontal when |dx| / (w + 40) >= |dy| / (h + 40) (centre to centre, w and h
//   the two cards' mean size), otherwise vertical. The edge leaves the source's side that
//   faces the target and enters the target's side that faces the source, at the side
//   midpoints.
// - Curve: a cubic Bézier whose control points extend along each side's outward normal by
//   k = max(36, 0.45 × the gap along that axis).

export type Pt = { x: number; y: number };
/** A card's box: top-left corner and size. */
export type Rect = { x: number; y: number; w: number; h: number };
export type Side = "left" | "right" | "top" | "bottom";

export type Route = {
  p0: Pt;
  p1: Pt;
  p2: Pt;
  p3: Pt;
  sourceSide: Side;
  targetSide: Side;
};

export const SIDE_MARGIN = 40;
export const MIN_K = 36;
export const K_FACTOR = 0.45;

const NORMAL: Record<Side, Pt> = {
  left: { x: -1, y: 0 },
  right: { x: 1, y: 0 },
  top: { x: 0, y: -1 },
  bottom: { x: 0, y: 1 },
};

export function centre(r: Rect): Pt {
  return { x: r.x + r.w / 2, y: r.y + r.h / 2 };
}

/** A point on one side of a card, `t` along it (0 = left/top end, 1 = right/bottom end). */
export function sidePoint(r: Rect, side: Side, t: number): Pt {
  switch (side) {
    case "left":
      return { x: r.x, y: r.y + r.h * t };
    case "right":
      return { x: r.x + r.w, y: r.y + r.h * t };
    case "top":
      return { x: r.x + r.w * t, y: r.y };
    case "bottom":
      return { x: r.x + r.w * t, y: r.y + r.h };
  }
}

/** The midpoint of one side of a card. */
export function sideMidpoint(r: Rect, side: Side): Pt {
  return sidePoint(r, side, 0.5);
}

/** Where an edge end sits when several share a side (F1): `slot` of `count`. */
export type Slot = { slot: number; count: number };

/** Ends sharing a side spread evenly within its middle 70%; a single end stays centred. */
export function slotPosition(s?: Slot): number {
  if (!s || s.count <= 1) return 0.5;
  return 0.15 + (0.7 * (s.slot + 0.5)) / s.count;
}

/** Which sides face each other: `a`'s side towards `b`, and `b`'s side towards `a`. */
export function facingSides(a: Rect, b: Rect): { source: Side; target: Side } {
  const ac = centre(a);
  const bc = centre(b);
  const dx = bc.x - ac.x;
  const dy = bc.y - ac.y;
  const w = (a.w + b.w) / 2;
  const h = (a.h + b.h) / 2;
  const horizontal = Math.abs(dx) / (w + SIDE_MARGIN) >= Math.abs(dy) / (h + SIDE_MARGIN);
  if (horizontal) {
    return dx >= 0 ? { source: "right", target: "left" } : { source: "left", target: "right" };
  }
  return dy >= 0 ? { source: "bottom", target: "top" } : { source: "top", target: "bottom" };
}

/** The curve from card `a` to card `b`, its ends at their slots (default: the side
 * midpoints). */
export function routeEdge(a: Rect, b: Rect, slots?: { source?: Slot; target?: Slot }): Route {
  const { source, target } = facingSides(a, b);
  const p0 = sidePoint(a, source, slotPosition(slots?.source));
  const p3 = sidePoint(b, target, slotPosition(slots?.target));
  const horizontal = source === "left" || source === "right";
  const gap = horizontal ? Math.abs(p3.x - p0.x) : Math.abs(p3.y - p0.y);
  const k = Math.max(MIN_K, K_FACTOR * gap);
  const ns = NORMAL[source];
  const nt = NORMAL[target];
  return {
    p0,
    p1: { x: p0.x + ns.x * k, y: p0.y + ns.y * k },
    p2: { x: p3.x + nt.x * k, y: p3.y + nt.y * k },
    p3,
    sourceSide: source,
    targetSide: target,
  };
}

/** A point on the curve, t in [0, 1]. */
export function bezierPoint(r: Pick<Route, "p0" | "p1" | "p2" | "p3">, t: number): Pt {
  const u = 1 - t;
  const a = u * u * u;
  const b = 3 * u * u * t;
  const c = 3 * u * t * t;
  const d = t * t * t;
  return {
    x: a * r.p0.x + b * r.p1.x + c * r.p2.x + d * r.p3.x,
    y: a * r.p0.y + b * r.p1.y + c * r.p2.y + d * r.p3.y,
  };
}

/** The direction of travel at t, in radians (0 = pointing right). */
export function bezierAngle(r: Pick<Route, "p0" | "p1" | "p2" | "p3">, t: number): number {
  const u = 1 - t;
  // Derivative of the cubic Bézier.
  const dx = 3 * u * u * (r.p1.x - r.p0.x) + 6 * u * t * (r.p2.x - r.p1.x) + 3 * t * t * (r.p3.x - r.p2.x);
  const dy = 3 * u * u * (r.p1.y - r.p0.y) + 6 * u * t * (r.p2.y - r.p1.y) + 3 * t * t * (r.p3.y - r.p2.y);
  return Math.atan2(dy, dx);
}

/** The SVG path of the curve. */
export function toPath(r: Pick<Route, "p0" | "p1" | "p2" | "p3">): string {
  const f = (p: Pt) => `${round(p.x)},${round(p.y)}`;
  return `M${f(r.p0)} C${f(r.p1)} ${f(r.p2)} ${f(r.p3)}`;
}

const round = (n: number) => Math.round(n * 100) / 100;
