// Chaos events as the UI injects them. On the wire a selector is flattened into the event
// (`{ "kill": { "id": "db", "at": 12 } }`, the same shape problem files use), but the
// generated binding types it as a nested `target` (specta doesn't render serde's flatten), so
// the UI builds events only here and casts once.
import type { ChaosEvent } from "@/bindings";

export type ChaosKind = "kill" | "latency" | "partition" | "traffic" | "flush" | "external";

const make = (kind: ChaosKind, body: Record<string, unknown>) => ({ [kind]: { at: 0, ...body } }) as unknown as ChaosEvent;

/** Injected events start at 0; the run store moves them to the moment of injection. */
export const chaos = {
  kill: (id: string) => make("kill", { id }),
  latency: (id: string, addMs: number, forS: number) => make("latency", { id, add_ms: addMs, for: forS }),
  flush: (id: string) => make("flush", { id }),
  traffic: (x: number, forS: number) => make("traffic", { x, for: forS }),
};

export function chaosKind(e: ChaosEvent): ChaosKind {
  return Object.keys(e)[0] as ChaosKind;
}
