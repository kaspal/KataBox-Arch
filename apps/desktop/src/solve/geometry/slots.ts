// Edge slots (F1): when several edges attach to the same side of a card, they spread
// along it instead of all meeting at the midpoint. Ends are ordered by where the far end
// sits along that side, so neighbouring edges don't cross. Pure, unit-tested; the packet
// layer uses the same slots.
import { centre, facingSides, type Rect, type Side, type Slot } from "@/solve/geometry/routeEdge";

export type EdgeEnds = { id: string; from: string; to: string };
export type EdgeSlots = { source: Slot; target: Slot };

type End = { edge: string; end: "source" | "target"; key: string; side: Side; far: number };

/** Slots for every edge whose two cards have a box in \`rects\`. */
export function assignSlots(edges: EdgeEnds[], rects: ReadonlyMap<string, Rect>): Map<string, EdgeSlots> {
  const groups = new Map<string, End[]>();
  const push = (e: End) => {
    const list = groups.get(e.key);
    if (list) list.push(e);
    else groups.set(e.key, [e]);
  };
  for (const e of edges) {
    const a = rects.get(e.from);
    const b = rects.get(e.to);
    if (!a || !b) continue;
    const { source, target } = facingSides(a, b);
    const along = (side: Side, other: Rect) => {
      const c = centre(other);
      return side === "top" || side === "bottom" ? c.x : c.y;
    };
    push({ edge: e.id, end: "source", key: `${e.from}:${source}`, side: source, far: along(source, b) });
    push({ edge: e.id, end: "target", key: `${e.to}:${target}`, side: target, far: along(target, a) });
  }
  const out = new Map<string, EdgeSlots>();
  for (const list of groups.values()) {
    list.sort((x, y) => x.far - y.far || (x.edge < y.edge ? -1 : x.edge > y.edge ? 1 : 0) || (x.end < y.end ? -1 : 1));
    list.forEach((e, i) => {
      const slots = out.get(e.edge) ?? { source: { slot: 0, count: 1 }, target: { slot: 0, count: 1 } };
      slots[e.end] = { slot: i, count: list.length };
      out.set(e.edge, slots);
    });
  }
  return out;
}
