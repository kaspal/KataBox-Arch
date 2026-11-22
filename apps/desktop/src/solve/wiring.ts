// Instant wiring feedback (ADR 0004). The rules are not here: they are the table Rust
// sends inside the registry (`ka-core/src/wiring.rs`). This is the same small evaluator
// Rust runs over that table; `wiring.test.ts` checks it reproduces Rust's exported
// kind × kind matrix exactly. Rust re-validates every design on save.
import type { ComponentKind, Edge, KindSet, Node, Protocol, Registry, WiringReason, WiringTable } from "@/bindings";

/** `Protocol`'s declaration order in Rust (results are sorted by it, as in Rust). */
export const PROTOCOLS: Protocol[] = ["http", "grpc", "ws", "query", "publish", "consume", "replicate"];

/** Async messages are dashed; replication is dotted; sync calls are solid. */
export function protocolStyle(p: Protocol): "solid" | "dashed" | "dotted" {
  if (p === "publish" || p === "consume") return "dashed";
  if (p === "replicate") return "dotted";
  return "solid";
}

export type Wiring = {
  allowed: (from: ComponentKind, to: ComponentKind) => Protocol[];
  reason: (from: ComponentKind, to: ComponentKind) => WiringReason;
};

export function makeWiring(registry: Pick<Registry, "kinds" | "wiring">): Wiring {
  const category = new Map(registry.kinds.map((k) => [k.kind, k.category]));
  const table: WiringTable = registry.wiring;
  const contains = (set: KindSet, kind: ComponentKind) =>
    !set.except.includes(kind) && (set.categories.includes(category.get(kind)!) || set.kinds.includes(kind));
  const allowed = (from: ComponentKind, to: ComponentKind) => {
    const found = new Set<Protocol>();
    for (const r of table.rules) {
      if (contains(r.from, from) && contains(r.to, to) && (!r.same_kind || from === to)) found.add(r.protocol);
    }
    return PROTOCOLS.filter((p) => found.has(p));
  };
  const reason = (from: ComponentKind, to: ComponentKind): WiringReason =>
    table.denials.find((d) => contains(d.from, from) && contains(d.to, to))?.reason ?? "no_rule";
  return { allowed, reason };
}

export type WireCheck = { ok: true; protocols: Protocol[] } | { ok: false; reason: WiringReason };

/** May a wire go from card `fromId` to card `toId`? Adds the card-level checks the kind
 * table can't see: self-loops, and protocols already used for this pair. */
export function checkWire(
  wiring: Wiring,
  nodes: Pick<Node, "id" | "kind">[],
  edges: Pick<Edge, "from" | "to" | "protocol">[],
  fromId: string,
  toId: string,
): WireCheck {
  if (fromId === toId) return { ok: false, reason: "self_loop" };
  const from = nodes.find((n) => n.id === fromId);
  const to = nodes.find((n) => n.id === toId);
  if (!from || !to) return { ok: false, reason: "no_rule" };
  const allowed = wiring.allowed(from.kind, to.kind);
  if (allowed.length === 0) return { ok: false, reason: wiring.reason(from.kind, to.kind) };
  const used = new Set(edges.filter((e) => e.from === fromId && e.to === toId).map((e) => e.protocol));
  const free = allowed.filter((p) => !used.has(p));
  return free.length > 0 ? { ok: true, protocols: free } : { ok: false, reason: "duplicate_edge" };
}
