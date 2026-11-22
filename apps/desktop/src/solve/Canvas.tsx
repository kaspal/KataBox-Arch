// The free-form canvas (ADR 0004): React Flow drawing the editor store. Cards go
// anywhere (8 px snapping, Alt to place freely), edges are curves that follow, wiring is
// checked against Rust's rule table as you drag, and everything works from the keyboard.
import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import {
  Background,
  BackgroundVariant,
  ConnectionMode,
  MiniMap,
  ReactFlow,
  ReactFlowProvider,
  useReactFlow,
  useViewport,
  type Connection,
  type Edge as RFEdge,
  type EdgeChange,
  type NodeChange,
  type NodeTypes,
  type EdgeTypes,
} from "@xyflow/react";
import { useTranslation } from "react-i18next";
import { Maximize, Minus, Plus } from "lucide-react";
import type { ComponentKind, Flavour, Point, Protocol, Registry } from "@/bindings";
import { Button } from "@/components/ui/button";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { useSnap } from "@/lib/settings";
import { CanvasContext, useCanvas } from "@/solve/context";
import { ConnectionLine } from "@/solve/edges/ConnectionLine";
import { CurvedEdge, type CurvedFlowEdge } from "@/solve/edges/CurvedEdge";
import { FloatingPicker } from "@/solve/FloatingPicker";
import { assignSlots } from "@/solve/geometry/slots";
import { cycle, nearestInDirection, type Direction, type Placed } from "@/solve/keyboardWiring";
import { registerCanvasAdd, type AddRequest } from "@/solve/bridge";
import { ComponentCard, type CardFlowNode } from "@/solve/nodes/ComponentCard";
import { PacketLayer } from "@/solve/run/PacketLayer";
import { NoteNode, type NoteFlowNode } from "@/solve/nodes/NoteNode";
import { ZoneNode, type ZoneFlowNode } from "@/solve/nodes/ZoneNode";
import { defaultProps, flavourName, kindName, kindSpec } from "@/solve/registry";
import { CARD_H, CARD_W, NOTE_SIZE, ZONE_SIZE, useEditor } from "@/solve/store";
import { checkWire, makeWiring } from "@/solve/wiring";

export const KIND_MIME = "application/x-katabox-arch-item";
export const GRID = 8;
/** "Fit the design" never zooms out past this: card text stays readable (F5). */
export const FIT_MIN_ZOOM = 0.8;

/** True once `el` has a size. React Flow warns when it mounts in a 0×0 box (F6), which
 * happens for a moment while the resizable panels measure themselves. */
function useHasSize(ref: React.RefObject<HTMLElement | null>): boolean {
  const [ready, setReady] = useState(false);
  useLayoutEffect(() => {
    const el = ref.current;
    if (!el) return;
    const check = () => el.clientWidth > 0 && el.clientHeight > 0;
    if (check()) {
      setReady(true);
      return;
    }
    const ro = new ResizeObserver(() => {
      if (check()) {
        setReady(true);
        ro.disconnect();
      }
    });
    ro.observe(el);
    return () => ro.disconnect();
  }, [ref]);
  return ready;
}

const nodeTypes: NodeTypes = { component: ComponentCard, zone: ZoneNode, note: NoteNode };
const edgeTypes: EdgeTypes = { curved: CurvedEdge };

type FlowNode = CardFlowNode | ZoneFlowNode | NoteFlowNode;

type Picker =
  | { type: "flavour"; kind: ComponentKind; at: Point; screen: Point }
  | { type: "protocol"; from: string; to: string; protocols: Protocol[]; screen: Point };

const snapTo = (v: number) => Math.round(v / GRID) * GRID;

function useAltKey() {
  const [alt, setAlt] = useState(false);
  useEffect(() => {
    const on = (e: KeyboardEvent) => setAlt(e.altKey);
    const off = () => setAlt(false);
    window.addEventListener("keydown", on);
    window.addEventListener("keyup", on);
    window.addEventListener("blur", off);
    return () => {
      window.removeEventListener("keydown", on);
      window.removeEventListener("keyup", on);
      window.removeEventListener("blur", off);
    };
  }, []);
  return alt;
}

const isTyping = (t: EventTarget | null) =>
  t instanceof HTMLElement && (t.isContentEditable || ["INPUT", "TEXTAREA", "SELECT"].includes(t.tagName));

export function Canvas({ registry }: { registry: Registry }) {
  const wiring = useMemo(() => makeWiring(registry), [registry]);
  const ctx = useMemo(() => ({ registry, wiring }), [registry, wiring]);
  return (
    <CanvasContext.Provider value={ctx}>
      <ReactFlowProvider>
        <CanvasInner />
      </ReactFlowProvider>
    </CanvasContext.Provider>
  );
}

function CanvasInner() {
  const { t } = useTranslation();
  const { registry, wiring } = useCanvas();
  const doc = useEditor((s) => s.doc);
  const selection = useEditor((s) => s.selection);
  const store = useEditor.getState;
  const rf = useReactFlow<FlowNode, CurvedFlowEdge>();
  const [snap] = useSnap();
  const alt = useAltKey();
  const wrapper = useRef<HTMLDivElement>(null);
  const sizes = useRef(new Map<string, { w: number; h: number }>());
  // Measured sizes, handed back to React Flow on each node (the minimap and fitView read
  // them from the nodes we pass in).
  const [measured, setMeasured] = useState<ReadonlyMap<string, { width: number; height: number }>>(new Map());
  const [picker, setPicker] = useState<Picker | null>(null);
  const [connect, setConnect] = useState<{ from: string; target: string | null } | null>(null);
  const [announce, setAnnounce] = useState("");
  const snapping = snap && !alt;
  const sized = useHasSize(wrapper);

  // --- What React Flow draws -------------------------------------------------------
  const selected = useMemo(() => new Set(selection), [selection]);
  const connectCheck = useMemo(
    () => (connect?.target ? checkWire(wiring, doc.nodes, doc.edges, connect.from, connect.target) : null),
    [connect, wiring, doc.nodes, doc.edges],
  );
  const nodes = useMemo<FlowNode[]>(() => {
    const zones: ZoneFlowNode[] = doc.groups.map((g) => ({
      id: g.id,
      type: "zone",
      position: g.position,
      data: { group: g },
      width: g.size.w,
      measured: { width: g.size.w, height: g.size.h },
      height: g.size.h,
      style: { width: g.size.w, height: g.size.h },
      className: "kb-zone",
      dragHandle: ".kb-zone-handle",
      selected: selected.has(g.id),
      zIndex: g.kind === "region" ? -2 : -1,
      ariaLabel: `${t(`canvas.zone.${g.kind}`)} ${g.label}`.trim(),
    }));
    const notes: NoteFlowNode[] = doc.notes.map((n) => ({
      id: n.id,
      type: "note",
      position: n.position,
      data: { note: n, maxLength: registry.limits.max_note },
      width: n.size.w,
      measured: { width: n.size.w, height: n.size.h },
      height: n.size.h,
      style: { width: n.size.w, height: n.size.h },
      selected: selected.has(n.id),
      ariaLabel: t("canvas.noteAria", { text: n.text.slice(0, 60) }),
    }));
    const cards: CardFlowNode[] = doc.nodes.map((n) => {
      const mark =
        connect?.from === n.id
          ? ("source" as const)
          : connect?.target === n.id
            ? connectCheck?.ok
              ? ("target-ok" as const)
              : ("target-blocked" as const)
            : undefined;
      return {
        id: n.id,
        type: "component",
        position: n.position,
        data: { node: n, spec: kindSpec(registry, n.kind), wiring: mark },
        selected: selected.has(n.id),
        measured: measured.get(n.id),
        ariaLabel: `${n.label.trim() || flavourName(t, n.flavour)}, ${kindName(t, n.kind)}`,
      };
    });
    return [...zones, ...notes, ...cards];
  }, [doc, selected, connect, connectCheck, registry, t, measured]);

  // Edges sharing a side of a card spread into slots, from the cards' live boxes (F1).
  const slots = useMemo(() => {
    const rects = new Map(
      doc.nodes.map((n) => {
        const m = measured.get(n.id);
        return [n.id, { x: n.position.x, y: n.position.y, w: m?.width ?? CARD_W, h: m?.height ?? CARD_H }];
      }),
    );
    return assignSlots(doc.edges, rects);
  }, [doc.nodes, doc.edges, measured]);

  const edges = useMemo<CurvedFlowEdge[]>(
    () =>
      doc.edges.map((e) => ({
        id: e.id,
        source: e.from,
        target: e.to,
        type: "curved",
        data: { protocol: e.protocol, slots: slots.get(e.id) },
        selected: selected.has(e.id),
      })),
    [doc.edges, selected, slots],
  );

  // --- Changes from React Flow ----------------------------------------------------
  const onNodesChange = useCallback(
    (changes: NodeChange<FlowNode>[]) => {
      const s = store();
      const moves: { id: string; position: Point }[] = [];
      const removed: string[] = [];
      let sel: Set<string> | null = null;
      let sized = false;
      for (const c of changes) {
        if (c.type === "position" && c.position) {
          moves.push({ id: c.id, position: { x: c.position.x, y: c.position.y } });
        } else if (c.type === "dimensions" && c.dimensions) {
          // Only a real change re-renders: a fresh map on every report feeds React Flow's
          // ResizeObserver back into itself (a loop, 60 times a second, with zones on screen).
          const prev = sizes.current.get(c.id);
          if (!prev || prev.w !== c.dimensions.width || prev.h !== c.dimensions.height) {
            sizes.current.set(c.id, { w: c.dimensions.width, h: c.dimensions.height });
            sized = true;
          }
          if (c.setAttributes) {
            const size = { w: c.dimensions.width, h: c.dimensions.height };
            if (s.doc.groups.some((g) => g.id === c.id)) s.updateGroup(c.id, { size });
            else if (s.doc.notes.some((n) => n.id === c.id)) s.updateNote(c.id, { size });
          }
        } else if (c.type === "select") {
          sel ??= new Set(s.selection);
          if (c.selected) sel.add(c.id);
          else sel.delete(c.id);
        } else if (c.type === "remove") {
          removed.push(c.id);
        }
      }
      if (sized) {
        setMeasured(new Map([...sizes.current].map(([id, z]) => [id, { width: z.w, height: z.h }])));
      }
      if (moves.length) s.moveItems(moves);
      if (sel) s.setSelection([...sel]);
      if (removed.length) s.removeItems(removed);
    },
    [store],
  );

  const onEdgesChange = useCallback(
    (changes: EdgeChange<CurvedFlowEdge>[]) => {
      const s = store();
      let sel: Set<string> | null = null;
      const removed: string[] = [];
      for (const c of changes) {
        if (c.type === "select") {
          sel ??= new Set(s.selection);
          if (c.selected) sel.add(c.id);
          else sel.delete(c.id);
        } else if (c.type === "remove") {
          removed.push(c.id);
        }
      }
      if (sel) s.setSelection([...sel]);
      if (removed.length) s.removeItems(removed);
    },
    [store],
  );

  const beginDrag = useCallback(() => store().beginDrag(), [store]);
  const endDrag = useCallback(() => store().endDrag(sizes.current), [store]);

  // --- Positions -------------------------------------------------------------------
  const toLocal = useCallback((screen: Point): Point => {
    const r = wrapper.current?.getBoundingClientRect();
    return { x: screen.x - (r?.left ?? 0), y: screen.y - (r?.top ?? 0) };
  }, []);

  const viewportCentre = useCallback((): { flow: Point; local: Point } => {
    const r = wrapper.current?.getBoundingClientRect();
    const screen = { x: (r?.left ?? 0) + (r?.width ?? 800) / 2, y: (r?.top ?? 0) + (r?.height ?? 600) / 2 };
    return { flow: rf.screenToFlowPosition(screen), local: toLocal(screen) };
  }, [rf, toLocal]);

  /** Top-left for an item of size `size` centred on `p`, snapped when snapping is on. */
  const placeAt = useCallback(
    (p: Point, size: { w: number; h: number }): Point => {
      const x = p.x - size.w / 2;
      const y = p.y - size.h / 2;
      return snapping ? { x: snapTo(x), y: snapTo(y) } : { x, y };
    },
    [snapping],
  );

  /** A card, or for a wrapper kind (Autoscaling) a frame holding its policy. */
  const placeKind = useCallback(
    (kind: ComponentKind, flavour: Flavour, centre: Point) => {
      const spec = kindSpec(registry, kind);
      if (spec.placement === "wrapper") {
        store().addGroup("autoscale", placeAt(centre, ZONE_SIZE.autoscale), { flavour, props: defaultProps(spec, flavour) });
      } else {
        store().addNode(registry, kind, flavour, placeAt(centre, { w: CARD_W, h: CARD_H }));
      }
    },
    [placeAt, registry, store],
  );

  const add = useCallback(
    (req: AddRequest, at?: { flow: Point; local: Point }) => {
      const where = at ?? viewportCentre();
      const s = store();
      if ("pattern" in req) {
        s.insertDesign(req.pattern, where.flow);
      } else if ("kind" in req) {
        if (req.pick) {
          setPicker({ type: "flavour", kind: req.kind, at: where.flow, screen: where.local });
        } else {
          placeKind(req.kind, kindSpec(registry, req.kind).default_flavour, where.flow);
        }
      } else if (req.meta === "note") {
        s.addNote(placeAt(where.flow, NOTE_SIZE));
      } else {
        s.addGroup(req.meta, placeAt(where.flow, ZONE_SIZE[req.meta]));
      }
    },
    [placeAt, placeKind, registry, store, viewportCentre],
  );

  // Take add requests (palette, patterns) only once React Flow has its viewport: before
  // that, screen-to-flow conversion is wrong and inserted items land off screen.
  const [rfReady, setRfReady] = useState(false);
  useEffect(() => (rfReady ? registerCanvasAdd((req) => add(req)) : undefined), [add, rfReady]);

  // --- Drag and drop from the library ----------------------------------------------
  const onDragOver = (e: React.DragEvent) => {
    if (!e.dataTransfer.types.includes(KIND_MIME)) return;
    e.preventDefault();
    e.dataTransfer.dropEffect = "copy";
  };
  const onDrop = (e: React.DragEvent) => {
    const raw = e.dataTransfer.getData(KIND_MIME);
    if (!raw) return;
    e.preventDefault();
    const req = JSON.parse(raw) as AddRequest;
    const screen = { x: e.clientX, y: e.clientY };
    add({ ...req, pick: true }, { flow: rf.screenToFlowPosition(screen), local: toLocal(screen) });
  };

  // --- Wiring ----------------------------------------------------------------------
  const isValidConnection = useCallback(
    (c: Connection | RFEdge) => {
      const s = store();
      return checkWire(wiring, s.doc.nodes, s.doc.edges, c.source, c.target).ok;
    },
    [store, wiring],
  );

  const wire = useCallback(
    (from: string, to: string) => {
      const s = store();
      const check = checkWire(wiring, s.doc.nodes, s.doc.edges, from, to);
      if (!check.ok) {
        setAnnounce(t(`wiring.reason.${check.reason}`));
        return;
      }
      if (check.protocols.length === 1) {
        s.addEdge(from, to, check.protocols[0]);
        setAnnounce(t("wiring.added", { protocol: t(`protocol.${check.protocols[0]}`) }));
        return;
      }
      const target = rf.getInternalNode(to);
      const p = target ? target.internals.positionAbsolute : { x: 0, y: 0 };
      const screen = rf.flowToScreenPosition({ x: p.x + (target?.measured.width ?? CARD_W) / 2, y: p.y });
      setPicker({ type: "protocol", from, to, protocols: check.protocols, screen: toLocal(screen) });
    },
    [rf, store, t, toLocal, wiring],
  );

  const onConnect = useCallback((c: Connection) => wire(c.source, c.target), [wire]);

  // --- Keyboard wiring -------------------------------------------------------------
  const placed = useCallback((): Placed[] => {
    return store().doc.nodes.map((n) => {
      const size = sizes.current.get(n.id) ?? { w: CARD_W, h: CARD_H };
      return { id: n.id, x: n.position.x + size.w / 2, y: n.position.y + size.h / 2 };
    });
  }, [store]);

  const describeTarget = useCallback(
    (from: string, target: string | null) => {
      const s = store();
      const n = s.doc.nodes.find((x) => x.id === target);
      if (!n || !target) return;
      const name = n.label.trim() || flavourName(t, n.flavour);
      const check = checkWire(wiring, s.doc.nodes, s.doc.edges, from, target);
      setAnnounce(
        check.ok
          ? t("wiring.keyboard.target", {
              name,
              protocols: check.protocols.map((p) => t(`protocol.${p}`)).join(", "),
            })
          : t("wiring.keyboard.blocked", { name, reason: t(`wiring.reason.${check.reason}`) }),
      );
    },
    [store, t, wiring],
  );

  const onKeyDownCapture = (e: React.KeyboardEvent) => {
    if (isTyping(e.target) || picker) return;
    const s = store();
    if (!connect) {
      const only = s.selection.length === 1 ? s.doc.nodes.find((n) => n.id === s.selection[0]) : undefined;
      if ((e.key === "c" || e.key === "C") && !e.ctrlKey && !e.metaKey && !e.altKey && only) {
        e.preventDefault();
        e.stopPropagation();
        setConnect({ from: only.id, target: null });
        setAnnounce(t("wiring.keyboard.start", { name: only.label.trim() || flavourName(t, only.flavour) }));
      }
      return;
    }
    const arrows: Record<string, Direction> = { ArrowUp: "up", ArrowDown: "down", ArrowLeft: "left", ArrowRight: "right" };
    let next: string | null | undefined;
    if (e.key in arrows) {
      const cards = placed();
      const at = cards.find((c) => c.id === (connect.target ?? connect.from));
      next = at ? (nearestInDirection(cards, at, arrows[e.key], [connect.from]) ?? connect.target) : null;
    } else if (e.key === "Tab") {
      next = cycle(placed(), connect.target, e.shiftKey ? -1 : 1, [connect.from]);
    } else if (e.key === "Enter") {
      e.preventDefault();
      e.stopPropagation();
      if (connect.target) {
        setConnect(null);
        wire(connect.from, connect.target);
      }
      return;
    } else if (e.key === "Escape") {
      e.preventDefault();
      e.stopPropagation();
      setConnect(null);
      setAnnounce(t("wiring.keyboard.cancelled"));
      return;
    } else {
      return;
    }
    e.preventDefault();
    e.stopPropagation();
    if (next) {
      setConnect({ from: connect.from, target: next });
      describeTarget(connect.from, next);
    }
  };

  // Cancel keyboard wiring if its source card disappears.
  useEffect(() => {
    if (connect && !doc.nodes.some((n) => n.id === connect.from)) setConnect(null);
  }, [connect, doc.nodes]);

  // --- Picker results --------------------------------------------------------------
  const pickFlavour = (p: Extract<Picker, { type: "flavour" }>, flavour: Flavour) => {
    setPicker(null);
    placeKind(p.kind, flavour, p.at);
    wrapper.current?.focus();
  };
  const pickProtocol = (p: Extract<Picker, { type: "protocol" }>, protocol: Protocol) => {
    setPicker(null);
    store().addEdge(p.from, p.to, protocol);
    setAnnounce(t("wiring.added", { protocol: t(`protocol.${protocol}`) }));
    wrapper.current?.focus();
  };

  return (
    <div
      ref={wrapper}
      className="kb-canvas relative h-full w-full bg-canvas outline-none"
      onDragOver={onDragOver}
      onDrop={onDrop}
      onKeyDownCapture={onKeyDownCapture}
      tabIndex={-1}
      data-testid="canvas"
    >
      {sized && (
      <ReactFlow<FlowNode, CurvedFlowEdge>
        key={doc.id}
        nodes={nodes}
        edges={edges}
        nodeTypes={nodeTypes}
        edgeTypes={edgeTypes}
        onInit={() => setRfReady(true)}
        onNodesChange={onNodesChange}
        onEdgesChange={onEdgesChange}
        onNodeDragStart={beginDrag}
        onNodeDragStop={endDrag}
        onSelectionDragStart={beginDrag}
        onSelectionDragStop={endDrag}
        onConnect={onConnect}
        isValidConnection={isValidConnection}
        connectionMode={ConnectionMode.Loose}
        connectionLineComponent={ConnectionLine}
        connectionRadius={28}
        defaultViewport={store().viewport}
        onMoveEnd={(_, vp) => store().setViewport({ x: vp.x, y: vp.y, zoom: vp.zoom })}
        snapToGrid={snapping}
        snapGrid={[GRID, GRID]}
        elevateNodesOnSelect={false}
        elevateEdgesOnSelect={false}
        deleteKeyCode={["Delete", "Backspace"]}
        minZoom={0.2}
        maxZoom={2.5}
      >
        <Background variant={BackgroundVariant.Dots} gap={16} size={1.3} color="var(--canvas-dot)" bgColor="var(--canvas)" />
        <PacketLayer />
        <MiniMap
          pannable
          zoomable
          position="bottom-right"
          ariaLabel={t("canvas.minimap")}
          // Inline colours: the minimap styles each node inline, so CSS classes don't apply.
          nodeColor={(n) =>
            n.type === "component"
              ? `var(--cat-${(n as CardFlowNode).data.spec.category})`
              : n.type === "note"
                ? "var(--note)"
                : "transparent"
          }
          nodeStrokeColor={(n) => (n.type === "zone" ? "var(--zone-line)" : "transparent")}
          nodeStrokeWidth={3}
          bgColor="var(--card)"
          maskColor="color-mix(in srgb, var(--desk) 55%, transparent)"
        />
        <ZoomControls />
      </ReactFlow>
      )}

      {picker?.type === "flavour" && (
        <FloatingPicker
          title={t("canvas.pickFlavour", { kind: kindName(t, picker.kind) })}
          options={kindSpec(registry, picker.kind).flavours.map((f) => ({
            value: f.flavour,
            label: flavourName(t, f.flavour),
          }))}
          initial={kindSpec(registry, picker.kind).default_flavour}
          at={picker.screen}
          onPick={(f) => pickFlavour(picker, f)}
          onCancel={() => setPicker(null)}
        />
      )}
      {picker?.type === "protocol" && (
        <FloatingPicker
          title={t("canvas.pickProtocol")}
          options={picker.protocols.map((p) => ({ value: p, label: t(`protocol.${p}`) }))}
          at={picker.screen}
          onPick={(p) => pickProtocol(picker, p)}
          onCancel={() => setPicker(null)}
        />
      )}
      {connect && (
        <div className="pointer-events-none absolute left-1/2 top-3 z-10 -translate-x-1/2 rounded-full border bg-card px-3 py-1 text-xs shadow-sm">
          {t("wiring.keyboard.hint")}
        </div>
      )}
      <div className="sr-only" role="status" aria-live="polite" data-testid="canvas-announce">
        {announce}
      </div>
    </div>
  );
}

function ZoomControls() {
  const { t } = useTranslation();
  const rf = useReactFlow();
  const { zoom } = useViewport();
  const btn = "size-8 bg-card";
  return (
    <div className="absolute bottom-3 left-3 z-10 flex items-center gap-1.5" data-testid="zoom-controls">
      <Tooltip>
        <TooltipTrigger asChild>
          <Button variant="outline" size="icon" className={btn} aria-label={t("canvas.zoomOut")} onClick={() => void rf.zoomOut()}>
            <Minus aria-hidden className="size-4" />
          </Button>
        </TooltipTrigger>
        <TooltipContent>{t("canvas.zoomOut")}</TooltipContent>
      </Tooltip>
      <span className="flex h-8 min-w-14 items-center justify-center rounded-md border bg-card px-2 font-mono text-xs" aria-live="off">
        {Math.round(zoom * 100)}%
      </span>
      <Tooltip>
        <TooltipTrigger asChild>
          <Button variant="outline" size="icon" className={btn} aria-label={t("canvas.zoomIn")} onClick={() => void rf.zoomIn()}>
            <Plus aria-hidden className="size-4" />
          </Button>
        </TooltipTrigger>
        <TooltipContent>{t("canvas.zoomIn")}</TooltipContent>
      </Tooltip>
      <Tooltip>
        <TooltipTrigger asChild>
          <Button
            variant="outline"
            size="icon"
            className={btn}
            aria-label={t("canvas.fit")}
            onClick={() => void rf.fitView({ padding: 0.2, duration: 200, minZoom: FIT_MIN_ZOOM })}
          >
            <Maximize aria-hidden className="size-4" />
          </Button>
        </TooltipTrigger>
        <TooltipContent>{t("canvas.fit")}</TooltipContent>
      </Tooltip>
    </div>
  );
}
