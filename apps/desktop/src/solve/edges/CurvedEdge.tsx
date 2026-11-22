// The curved edge (ADR 0004): re-routed from both cards' live boxes every render, so it
// follows a card while it is dragged and its anchors switch sides as the card passes its
// neighbour. The protocol chip sits at the curve's midpoint (t = 0.5). During a live run with
// reduced motion (no packets), the chip also shows the connection's throughput.
import { memo } from "react";
import {
  EdgeLabelRenderer,
  useInternalNode,
  type Edge as FlowEdge,
  type EdgeProps,
  type InternalNode,
} from "@xyflow/react";
import { useTranslation } from "react-i18next";
import type { Protocol } from "@/bindings";
import { useReducedMotion } from "@/lib/motion";
import { cn } from "@/lib/utils";
import { bezierAngle, bezierPoint, routeEdge, toPath, type Rect } from "@/solve/geometry/routeEdge";
import type { EdgeSlots } from "@/solve/geometry/slots";
import { useTelemetry } from "@/solve/run/telemetry";
import { CARD_H, CARD_W, useEditor } from "@/solve/store";
import { protocolStyle } from "@/solve/wiring";

export type CurvedEdgeData = { protocol: Protocol; slots?: EdgeSlots; /** No live numbers (see CardData.still). */ still?: boolean };
export type CurvedFlowEdge = FlowEdge<CurvedEdgeData, "curved">;

export function nodeRect(n: InternalNode): Rect {
  const p = n.internals.positionAbsolute;
  return { x: p.x, y: p.y, w: n.measured.width ?? CARD_W, h: n.measured.height ?? CARD_H };
}

const DASH: Record<ReturnType<typeof protocolStyle>, string | undefined> = {
  solid: undefined,
  dashed: "7 6",
  dotted: "0.5 6",
};

const ARROW = 9;

export const CurvedEdge = memo(function CurvedEdge({ id, source, target, data, selected }: EdgeProps<CurvedFlowEdge>) {
  const { t, i18n } = useTranslation();
  const reduced = useReducedMotion();
  const rate = useTelemetry((s) => (reduced && !data?.still ? s.edgeRates[id] : undefined));
  const a = useInternalNode(source);
  const b = useInternalNode(target);
  const setSelection = useEditor((s) => s.setSelection);
  if (!a || !b || !data) return null;
  const route = routeEdge(nodeRect(a), nodeRect(b), data.slots);
  const mid = bezierPoint(route, 0.5);
  // The arrowhead sits on the target's side, pointing along the curve's last tangent.
  const angle = bezierAngle(route, 1);
  const tip = route.p3;
  const left = { x: tip.x - ARROW * Math.cos(angle - 0.45), y: tip.y - ARROW * Math.sin(angle - 0.45) };
  const right = { x: tip.x - ARROW * Math.cos(angle + 0.45), y: tip.y - ARROW * Math.sin(angle + 0.45) };
  const style = protocolStyle(data.protocol);
  const stroke = selected ? "var(--primary)" : "var(--edge-line)";
  const label = t(`protocol.${data.protocol}`);
  const rateText =
    rate === undefined
      ? null
      : t("canvas.rate", { rate: new Intl.NumberFormat(i18n.language, { notation: "compact", maximumFractionDigits: 1 }).format(rate) });
  return (
    <>
      {/* A wide invisible stroke makes the curve easy to click. */}
      <path d={toPath(route)} fill="none" stroke="transparent" strokeWidth={14} className="react-flow__edge-interaction" />
      <path
        d={toPath(route)}
        fill="none"
        stroke={stroke}
        strokeWidth={selected ? 2 : 1.5}
        strokeDasharray={DASH[style]}
        strokeLinecap="round"
        data-testid="edge-path"
        data-protocol={data.protocol}
      />
      <path
        d={`M${left.x},${left.y} L${tip.x},${tip.y} L${right.x},${right.y}`}
        fill="none"
        stroke={stroke}
        strokeWidth={1.5}
        strokeLinecap="round"
        strokeLinejoin="round"
      />
      <EdgeLabelRenderer>
        <button
          type="button"
          className={cn(
            "nodrag nopan absolute rounded-full border bg-card px-2 py-0.5 font-mono text-[11px] text-muted-foreground shadow-[0_1px_0_var(--desk)]",
            selected ? "border-primary text-foreground" : "border-border",
          )}
          style={{ transform: `translate(-50%, -50%) translate(${mid.x}px, ${mid.y}px)`, pointerEvents: "all" }}
          onClick={() => setSelection([id])}
          aria-label={t("canvas.edgeLabel", { protocol: label })}
          data-testid="protocol-chip"
        >
          {label}
          {rateText && <span data-testid="edge-rate">{` · ${rateText}`}</span>}
        </button>
      </EdgeLabelRenderer>
    </>
  );
});
