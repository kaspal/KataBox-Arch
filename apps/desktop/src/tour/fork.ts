// "Try it yourself": a copy of a snapshot to edit in the Sandbox. Every id is new (so the
// copy never collides with the sample or an earlier copy), references follow the new ids,
// and the copy belongs to no problem.
import type { Design } from "@/bindings";
import { newId } from "@/solve/store";

export function forkDesign(d: Design, name: string): Design {
  const ids = new Map<string, string>();
  const fresh = (old: string, prefix: string) => {
    const id = newId(prefix);
    ids.set(old, id);
    return id;
  };
  const groups = d.groups.map((g) => ({ ...g, id: fresh(g.id, "group") }));
  const nodes = d.nodes.map((n) => ({ ...n, id: fresh(n.id, "node") }));
  const map = (id: string | null) => (id === null ? null : (ids.get(id) ?? null));
  return {
    ...structuredClone(d),
    id: newId("design"),
    name,
    problem_id: null,
    groups: groups.map((g) => ({ ...structuredClone(g), parent_id: map(g.parent_id) })),
    nodes: nodes.map((n) => ({ ...structuredClone(n), group_id: map(n.group_id) })),
    edges: d.edges.map((e) => ({ ...e, id: newId("edge"), from: ids.get(e.from) ?? e.from, to: ids.get(e.to) ?? e.to })),
    notes: d.notes.map((n) => ({ ...n, id: newId("note") })),
  };
}
