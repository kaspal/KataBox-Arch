// The design being edited (ADR 0004). Zustand holds it; zundo keeps the undo history of
// the design only (not the selection or the viewport). One drag, of one card, several
// cards or a zone with its members, is one undo step: history is paused while dragging
// and the whole move is recorded when the drag ends.
//
// The store is canvas-local state: nothing here touches files. Opening and saving go
// through `files.ts` → `api` → Rust, which validates.
import { create } from "zustand";
import { temporal } from "zundo";
import type {
  ComponentKind,
  Design,
  Edge,
  Flavour,
  Group,
  GroupKind,
  Node,
  Note,
  Point,
  PropValue,
  Protocol,
  Registry,
  Scaling,
  Viewport,
} from "@/bindings";

/** The `.kbarch` format version (ADR 0003); a test keeps it equal to Rust's. */
export const SCHEMA_VERSION = 4;

/** The card size the canvas draws (also used before React Flow has measured a card). */
export const CARD_W = 208;
export const CARD_H = 64;
export const NOTE_SIZE = { w: 200, h: 120 };
export const ZONE_SIZE: Record<GroupKind, { w: number; h: number }> = {
  region: { w: 640, h: 420 },
  az: { w: 360, h: 260 },
  autoscale: { w: 300, h: 160 },
};

/** Draw order (behind the cards): regions, then AZs, then autoscaling wrappers. */
const ZONE_ORDER: Record<GroupKind, number> = { region: 0, az: 1, autoscale: 2 };

/** Kinds an autoscaling wrapper scales; other cards in its frame join the zone around it. */
export const SCALABLE: ReadonlySet<ComponentKind> = new Set(["app", "micro", "worker", "serverless"]);

/** The design without the viewport: what undo and the dirty check look at. */
export type Doc = {
  id: string;
  name: string;
  problem_id: string | null;
  nodes: Node[];
  edges: Edge[];
  groups: Group[];
  notes: Note[];
};

export type EditorState = {
  doc: Doc;
  viewport: Viewport;
  /** The file this design was opened from or saved to; null until first saved. */
  path: string | null;
  /** `docJson(doc)` when last opened or saved: the dirty check compares against it. */
  savedJson: string;
  /** Selected item ids (cards, zones, notes, edges). */
  selection: string[];
  /** A drag is in progress (history paused). */
  dragging: boolean;
  /** H6: bumped to ask the canvas to fit the design in view (after a draft loads). */
  fitRequest: number;
};

type Actions = {
  load: (design: Design, path: string | null) => void;
  /** A design that came from elsewhere (a docker-compose import): open, never saved. */
  loadUnsaved: (design: Design) => void;
  newDesign: (name: string) => void;
  markSaved: (path: string) => void;
  rename: (name: string) => void;
  setViewport: (v: Viewport) => void;
  requestFit: () => void;
  setSelection: (ids: string[]) => void;

  addNode: (registry: Registry, kind: ComponentKind, flavour: Flavour, at: Point) => string;
  /** A zone; an autoscale wrapper also takes its scaling policy. */
  addGroup: (kind: GroupKind, at: Point, scaling?: Scaling) => string;
  updateScaling: (id: string, scaling: Scaling) => void;
  addNote: (at: Point) => string;
  /** Add every item of `design` (ids already fresh), centred on `centre`: one undo step. */
  insertDesign: (design: Design, centre: Point) => void;
  addEdge: (from: string, to: string, protocol: Protocol) => string;
  setEdgeProtocol: (id: string, protocol: Protocol) => void;
  removeItems: (ids: string[]) => void;

  updateNode: (id: string, patch: Partial<Pick<Node, "label">>) => void;
  setProp: (id: string, key: string, value: PropValue) => void;
  /** Drop a prop the user set, so the card follows its flavour's default again. */
  resetProp: (id: string, key: string) => void;
  setFlavour: (registry: Registry, id: string, flavour: Flavour) => void;
  updateGroup: (id: string, patch: Partial<Pick<Group, "label" | "size" | "position" | "region_tag" | "mesh">>) => void;
  updateNote: (id: string, patch: Partial<Pick<Note, "text" | "size" | "position">>) => void;

  beginDrag: () => void;
  /** Move items to new top-left positions. A zone's members follow it. */
  moveItems: (moves: { id: string; position: Point }[]) => void;
  /** Finish a drag: zone membership is recomputed, and the move becomes one undo step. */
  endDrag: (sizes?: Map<string, { w: number; h: number }>) => void;
};

export type EditorStore = EditorState & Actions;

/** A short random id suffix: [a-z0-9], valid for the Rust id rules. */
export function newId(prefix: string): string {
  const bytes = new Uint8Array(6);
  crypto.getRandomValues(bytes);
  return `${prefix}-${Array.from(bytes, (b) => (b % 36).toString(36)).join("")}`;
}

export function emptyDoc(name: string): Doc {
  return { id: newId("design"), name, problem_id: null, nodes: [], edges: [], groups: [], notes: [] };
}

/** The dirty-check form of a doc (key order as written, stable). */
export const docJson = (doc: Doc) => JSON.stringify(doc);

export function toDesign(s: Pick<EditorState, "doc" | "viewport">): Design {
  const { doc, viewport } = s;
  return {
    schema_version: SCHEMA_VERSION,
    id: doc.id,
    name: doc.name,
    problem_id: doc.problem_id,
    nodes: doc.nodes,
    edges: doc.edges,
    groups: doc.groups,
    notes: doc.notes,
    viewport,
  };
}

export function fromDesign(d: Design): Doc {
  return {
    id: d.id,
    name: d.name,
    problem_id: d.problem_id,
    nodes: d.nodes,
    edges: d.edges,
    groups: d.groups,
    notes: d.notes,
  };
}

const DEFAULT_VIEWPORT: Viewport = { x: 0, y: 0, zoom: 1 };

type Box = { x: number; y: number; w: number; h: number };

function contains(outer: Box, p: Point): boolean {
  return p.x >= outer.x && p.x <= outer.x + outer.w && p.y >= outer.y && p.y <= outer.y + outer.h;
}

/** The smallest zone (of `kinds`) whose box holds point `p`, excluding `self`. */
function innermostZone(groups: Group[], p: Point, kinds: GroupKind[], self?: string): Group | undefined {
  return groups
    .filter((g) => g.id !== self && kinds.includes(g.kind))
    .filter((g) => contains({ ...g.position, ...g.size }, p))
    .sort((a, b) => a.size.w * a.size.h - b.size.w * b.size.h)[0];
}

/** Every zone in `root`'s subtree (itself included). */
function zoneSubtree(groups: Group[], root: string): Set<string> {
  const out = new Set([root]);
  let grew = true;
  while (grew) {
    grew = false;
    for (const g of groups) {
      if (g.parent_id && out.has(g.parent_id) && !out.has(g.id)) {
        out.add(g.id);
        grew = true;
      }
    }
  }
  return out;
}

/** Recompute zone membership: an AZ joins the innermost region holding its centre, an
 * autoscaling wrapper the innermost AZ or region; a card joins the innermost zone holding
 * its centre, and only compute cards join autoscaling wrappers. */
export function withMembership(doc: Doc, sizes?: Map<string, { w: number; h: number }>): Doc {
  const PARENTS: Record<GroupKind, GroupKind[]> = { region: [], az: ["region"], autoscale: ["region", "az"] };
  const groups = doc.groups.map((g) => {
    const c = { x: g.position.x + g.size.w / 2, y: g.position.y + g.size.h / 2 };
    const kinds = PARENTS[g.kind];
    const parent = kinds.length ? (innermostZone(doc.groups, c, kinds, g.id)?.id ?? null) : null;
    return parent === g.parent_id ? g : { ...g, parent_id: parent };
  });
  const nodes = doc.nodes.map((n) => {
    const size = sizes?.get(n.id) ?? { w: CARD_W, h: CARD_H };
    const c = { x: n.position.x + size.w / 2, y: n.position.y + size.h / 2 };
    const kinds: GroupKind[] = SCALABLE.has(n.kind) ? ["region", "az", "autoscale"] : ["region", "az"];
    const zone = innermostZone(groups, c, kinds)?.id ?? null;
    return zone === n.group_id ? n : { ...n, group_id: zone };
  });
  const changed = groups.some((g, i) => g !== doc.groups[i]) || nodes.some((n, i) => n !== doc.nodes[i]);
  return changed ? { ...doc, groups, nodes } : doc;
}

let preDrag: Doc | null = null;

export const useEditor = create<EditorStore>()(
  temporal(
    (set, get) => {
      const patchDoc = (f: (doc: Doc) => Doc) => set((s) => ({ doc: f(s.doc) }));
      // A loaded or new design starts with no undo history.
      const clearHistory = () => useEditor.temporal.getState().clear();
      const initial = emptyDoc("Untitled design");
      return {
        doc: initial,
        viewport: DEFAULT_VIEWPORT,
        path: null,
        savedJson: docJson(initial),
        selection: [],
        dragging: false,
        fitRequest: 0,

        load: (design, path) => {
          const doc = fromDesign(design);
          set({ doc, viewport: design.viewport, path, savedJson: docJson(doc), selection: [] });
          clearHistory();
        },
        loadUnsaved: (design) => {
          const doc = fromDesign(design);
          // Nothing on disk matches it yet, so it counts as unsaved changes.
          set({ doc, viewport: design.viewport, path: null, savedJson: "", selection: [] });
          clearHistory();
        },
        newDesign: (name) => {
          const doc = emptyDoc(name);
          set({ doc, viewport: DEFAULT_VIEWPORT, path: null, savedJson: docJson(doc), selection: [] });
          clearHistory();
        },
        markSaved: (path) => set((s) => ({ path, savedJson: docJson(s.doc) })),
        rename: (name) => patchDoc((d) => ({ ...d, name })),
        setViewport: (viewport) => set({ viewport }),
        requestFit: () => set((s) => ({ fitRequest: s.fitRequest + 1 })),
        setSelection: (selection) => set({ selection }),

        addNode: (_registry, kind, flavour, at) => {
          const id = newId(kind);
          const node: Node = {
            id,
            kind,
            flavour,
            label: "",
            position: at,
            // Cards store only the props the user sets (`.kbarch` v3); the rest follow the
            // flavour's defaults, in the simulator and the inspector alike.
            props: {},
            group_id: null,
          };
          patchDoc((d) => withMembership({ ...d, nodes: [...d.nodes, node] }));
          set({ selection: [id] });
          return id;
        },
        addGroup: (kind, at, scaling) => {
          const id = newId(kind);
          const group: Group = { id, kind, label: "", position: at, size: ZONE_SIZE[kind], parent_id: null };
          if (kind === "autoscale" && scaling) group.scaling = scaling;
          // Zones draw behind cards: regions first, then AZs, then wrappers.
          patchDoc((d) => {
            const groups = [...d.groups, group].sort((a, b) => ZONE_ORDER[a.kind] - ZONE_ORDER[b.kind]);
            return withMembership({ ...d, groups });
          });
          set({ selection: [id] });
          return id;
        },
        updateScaling: (id, scaling) =>
          patchDoc((d) => ({ ...d, groups: d.groups.map((g) => (g.id === id ? { ...g, scaling } : g)) })),
        insertDesign: (design, centre) => {
          const xs = [...design.nodes.map((n) => [n.position.x, n.position.x + CARD_W]), ...design.groups.map((g) => [g.position.x, g.position.x + g.size.w]), ...design.notes.map((n) => [n.position.x, n.position.x + n.size.w])].flat();
          const ys = [...design.nodes.map((n) => [n.position.y, n.position.y + CARD_H]), ...design.groups.map((g) => [g.position.y, g.position.y + g.size.h]), ...design.notes.map((n) => [n.position.y, n.position.y + n.size.h])].flat();
          if (xs.length === 0) return;
          const dx = centre.x - (Math.min(...xs) + Math.max(...xs)) / 2;
          const dy = centre.y - (Math.min(...ys) + Math.max(...ys)) / 2;
          const at = (p: Point) => ({ x: Math.round(p.x + dx), y: Math.round(p.y + dy) });
          patchDoc((d) => {
            const groups = [...d.groups, ...design.groups.map((g) => ({ ...g, position: at(g.position) }))].sort((a, b) => ZONE_ORDER[a.kind] - ZONE_ORDER[b.kind]);
            return withMembership({
              ...d,
              groups,
              nodes: [...d.nodes, ...design.nodes.map((n) => ({ ...n, position: at(n.position) }))],
              edges: [...d.edges, ...design.edges],
              notes: [...d.notes, ...design.notes.map((n) => ({ ...n, position: at(n.position) }))],
            });
          });
          set({ selection: design.nodes.map((n) => n.id) });
        },
        addNote: (at) => {
          const id = newId("note");
          patchDoc((d) => ({ ...d, notes: [...d.notes, { id, text: "", position: at, size: NOTE_SIZE }] }));
          set({ selection: [id] });
          return id;
        },
        addEdge: (from, to, protocol) => {
          const id = newId("edge");
          patchDoc((d) => ({ ...d, edges: [...d.edges, { id, from, to, protocol }] }));
          return id;
        },
        setEdgeProtocol: (id, protocol) =>
          patchDoc((d) => ({ ...d, edges: d.edges.map((e) => (e.id === id ? { ...e, protocol } : e)) })),
        removeItems: (ids) => {
          const gone = new Set(ids);
          patchDoc((d) => {
            const nodes = d.nodes
              .filter((n) => !gone.has(n.id))
              .map((n) => (n.group_id && gone.has(n.group_id) ? { ...n, group_id: null } : n));
            const groups = d.groups
              .filter((g) => !gone.has(g.id))
              .map((g) => (g.parent_id && gone.has(g.parent_id) ? { ...g, parent_id: null } : g));
            const notes = d.notes.filter((n) => !gone.has(n.id));
            const edges = d.edges.filter((e) => !gone.has(e.id) && !gone.has(e.from) && !gone.has(e.to));
            return { ...d, nodes, groups, notes, edges };
          });
          set((s) => ({ selection: s.selection.filter((id) => !gone.has(id)) }));
        },

        updateNode: (id, patch) =>
          patchDoc((d) => ({ ...d, nodes: d.nodes.map((n) => (n.id === id ? { ...n, ...patch } : n)) })),
        setProp: (id, key, value) =>
          patchDoc((d) => ({
            ...d,
            nodes: d.nodes.map((n) => (n.id === id ? { ...n, props: { ...n.props, [key]: value } } : n)),
          })),
        resetProp: (id, key) =>
          patchDoc((d) => ({
            ...d,
            nodes: d.nodes.map((n) => {
              if (n.id !== id || !(key in n.props)) return n;
              const props = { ...n.props };
              delete props[key];
              return { ...n, props };
            }),
          })),
        setFlavour: (_registry, id, flavour) =>
          patchDoc((d) => ({
            ...d,
            // Props the user set are kept; the others follow the new flavour's defaults.
            nodes: d.nodes.map((n) => (n.id !== id || n.flavour === flavour ? n : { ...n, flavour })),
          })),
        updateGroup: (id, patch) =>
          patchDoc((d) => ({ ...d, groups: d.groups.map((g) => (g.id === id ? { ...g, ...patch } : g)) })),
        updateNote: (id, patch) =>
          patchDoc((d) => ({ ...d, notes: d.notes.map((n) => (n.id === id ? { ...n, ...patch } : n)) })),

        beginDrag: () => {
          if (get().dragging) return;
          preDrag = get().doc;
          useEditor.temporal.getState().pause();
          set({ dragging: true });
        },
        moveItems: (moves) =>
          patchDoc((d) => {
            const target = new Map(moves.map((m) => [m.id, m.position]));
            // A moved zone carries its members (sub-zones, cards) unless they move themselves.
            const carried = new Map<string, Point>();
            for (const g of d.groups) {
              const to = target.get(g.id);
              if (!to) continue;
              const delta = { x: to.x - g.position.x, y: to.y - g.position.y };
              if (delta.x === 0 && delta.y === 0) continue;
              const subtree = zoneSubtree(d.groups, g.id);
              for (const sub of d.groups) {
                if (sub.id !== g.id && subtree.has(sub.id) && !target.has(sub.id)) carried.set(sub.id, delta);
              }
              for (const n of d.nodes) {
                if (n.group_id && subtree.has(n.group_id) && !target.has(n.id)) carried.set(n.id, delta);
              }
            }
            const place = <T extends { id: string; position: Point }>(item: T): T => {
              const to = target.get(item.id);
              if (to) return to.x === item.position.x && to.y === item.position.y ? item : { ...item, position: to };
              const delta = carried.get(item.id);
              return delta ? { ...item, position: { x: item.position.x + delta.x, y: item.position.y + delta.y } } : item;
            };
            return { ...d, nodes: d.nodes.map(place), groups: d.groups.map(place), notes: d.notes.map(place) };
          }),
        endDrag: (sizes) => {
          if (!get().dragging) return;
          const final = withMembership(get().doc, sizes);
          const before = preDrag ?? final;
          preDrag = null;
          // Record the whole drag as one step: history sees `before` → `final`.
          set({ doc: before, dragging: false });
          useEditor.temporal.getState().resume();
          if (final !== before) set({ doc: final });
        },
      };
    },
    {
      partialize: (s) => ({ doc: s.doc }),
      equality: (a, b) => a.doc === b.doc,
      limit: 200,
    },
  ),
);

export const undo = () => useEditor.temporal.getState().undo();
export const redo = () => useEditor.temporal.getState().redo();

/** Has the design changed since it was opened or last saved? */
export function useDirty(): boolean {
  return useEditor((s) => docJson(s.doc) !== s.savedJson);
}

export function isDirty(): boolean {
  const s = useEditor.getState();
  return docJson(s.doc) !== s.savedJson;
}
