// A read-only canvas for a design that isn't the one being edited: the reference in Compare,
// and later the player's snapshots. It draws with the editor's card, zone, note and edge
// renderers, but nothing can be moved, wired or selected. The viewport is controlled by the
// caller, so two views can share one (linked zoom and pan).
import { useMemo, type ReactNode } from "react";
import {
  ReactFlow,
  ReactFlowProvider,
  type EdgeTypes,
  type NodeTypes,
  type Viewport,
} from "@xyflow/react";
import { useTranslation } from "react-i18next";
import type { Design, Registry } from "@/bindings";
import { CurvedEdge, type CurvedFlowEdge } from "@/solve/edges/CurvedEdge";
import { assignSlots } from "@/solve/geometry/slots";
import { ComponentCard, type CardFlowNode } from "@/solve/nodes/ComponentCard";
import { NoteNode, type NoteFlowNode } from "@/solve/nodes/NoteNode";
import { ZoneNode, type ZoneFlowNode } from "@/solve/nodes/ZoneNode";
import { flavourName, kindName, kindSpec } from "@/solve/registry";
import { CARD_H, CARD_W } from "@/solve/store";

type FlowNode = CardFlowNode | ZoneFlowNode | NoteFlowNode;

const nodeTypes: NodeTypes = { component: ComponentCard, zone: ZoneNode, note: NoteNode };
const edgeTypes: EdgeTypes = { curved: CurvedEdge };

export type Box = { x: number; y: number; w: number; h: number };

/** The design's bounding box in flow coordinates (cards at their nominal size). */
export function designBounds(d: Design): Box | null {
  const rects: Box[] = [
    ...d.nodes.map((n) => ({ x: n.position.x, y: n.position.y, w: CARD_W, h: CARD_H })),
    ...d.groups.map((g) => ({ x: g.position.x, y: g.position.y, w: g.size.w, h: g.size.h })),
    ...d.notes.map((n) => ({ x: n.position.x, y: n.position.y, w: n.size.w, h: n.size.h })),
  ];
  if (rects.length === 0) return null;
  const x0 = Math.min(...rects.map((r) => r.x));
  const y0 = Math.min(...rects.map((r) => r.y));
  const x1 = Math.max(...rects.map((r) => r.x + r.w));
  const y1 = Math.max(...rects.map((r) => r.y + r.h));
  return { x: x0, y: y0, w: x1 - x0, h: y1 - y0 };
}

/** A viewport that shows `box` inside a pane of `size`, with a margin, zoomed 0.3–1. */
export function fitBox(box: Box | null, size: { w: number; h: number }, margin = 32): Viewport {
  if (!box || size.w <= 0 || size.h <= 0) return { x: 0, y: 0, zoom: 1 };
  const zoom = Math.min(1, Math.max(0.3, Math.min((size.w - 2 * margin) / box.w, (size.h - 2 * margin) / box.h)));
  return {
    x: size.w / 2 - (box.x + box.w / 2) * zoom,
    y: size.h / 2 - (box.y + box.h / 2) * zoom,
    zoom,
  };
}

export function DesignView({
  design,
  registry,
  viewport,
  onViewportChange,
  label,
  interactive = true,
  focus,
  children,
  testId,
}: {
  design: Design;
  registry: Registry;
  viewport: Viewport;
  onViewportChange?: (v: Viewport) => void;
  /** Names the view for assistive technology ("Reference design"). */
  label: string;
  /** False: no pan or zoom (a ghost under another view). */
  interactive?: boolean;
  /** Cards in the spotlight; everything else dims. Empty or absent: nothing dims. */
  focus?: string[];
  /** Drawn inside the flow (the player's packet layer). */
  children?: ReactNode;
  testId?: string;
}) {
  const { t } = useTranslation();
  const lit = useMemo(() => (focus && focus.length > 0 ? new Set(focus) : null), [focus]);
  const nodes = useMemo<FlowNode[]>(() => {
    const zones: ZoneFlowNode[] = design.groups.map((g) => ({
      id: g.id,
      type: "zone",
      position: g.position,
      data: { group: g },
      width: g.size.w,
      height: g.size.h,
      measured: { width: g.size.w, height: g.size.h },
      style: { width: g.size.w, height: g.size.h },
      className: lit ? "kb-zone kb-dim" : "kb-zone",
      zIndex: g.kind === "region" ? -2 : -1,
      ariaLabel: `${t(`canvas.zone.${g.kind}`)} ${g.label}`.trim(),
    }));
    const notes: NoteFlowNode[] = design.notes.map((n) => ({
      id: n.id,
      type: "note",
      position: n.position,
      data: { note: n, maxLength: registry.limits.max_note },
      width: n.size.w,
      height: n.size.h,
      measured: { width: n.size.w, height: n.size.h },
      style: { width: n.size.w, height: n.size.h },
      className: lit ? "kb-dim" : undefined,
      ariaLabel: t("canvas.noteAria", { text: n.text.slice(0, 60) }),
    }));
    const cards: CardFlowNode[] = design.nodes.map((n) => ({
      id: n.id,
      type: "component",
      position: n.position,
      data: { node: n, spec: kindSpec(registry, n.kind) },
      measured: { width: CARD_W, height: CARD_H },
      className: lit && !lit.has(n.id) ? "kb-dim" : undefined,
      ariaLabel: `${n.label.trim() || flavourName(t, n.flavour)}, ${kindName(t, n.kind)}`,
    }));
    return [...zones, ...notes, ...cards];
  }, [design, registry, t, lit]);

  const edges = useMemo<CurvedFlowEdge[]>(() => {
    const rects = new Map(design.nodes.map((n) => [n.id, { x: n.position.x, y: n.position.y, w: CARD_W, h: CARD_H }]));
    const slots = assignSlots(design.edges, rects);
    return design.edges.map((e) => ({
      id: e.id,
      source: e.from,
      target: e.to,
      type: "curved",
      data: { protocol: e.protocol, slots: slots.get(e.id) },
      className: lit && !(lit.has(e.from) && lit.has(e.to)) ? "kb-dim" : undefined,
    }));
  }, [design, lit]);

  return (
    <div className="kb-canvas kb-readonly relative h-full w-full" role="group" aria-label={label} data-testid={testId}>
      <ReactFlowProvider>
        <ReactFlow<FlowNode, CurvedFlowEdge>
          nodes={nodes}
          edges={edges}
          nodeTypes={nodeTypes}
          edgeTypes={edgeTypes}
          viewport={viewport}
          onViewportChange={onViewportChange}
          nodesDraggable={false}
          nodesConnectable={false}
          nodesFocusable={false}
          edgesFocusable={false}
          elementsSelectable={false}
          panOnDrag={interactive}
          zoomOnScroll={interactive}
          zoomOnPinch={interactive}
          zoomOnDoubleClick={false}
          minZoom={0.3}
          maxZoom={2}
          proOptions={{ hideAttribution: true }}
        >
          {children}
        </ReactFlow>
      </ReactFlowProvider>
    </div>
  );
}
