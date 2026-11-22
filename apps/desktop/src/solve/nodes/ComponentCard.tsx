// The "index card" for a component: sheet body, 1px rule border, 12px radius, the kind's
// icon tile, the product (flavour) name, the kind, and one stat line: "—" until a run
// starts, then its live utilisation and queue, with a bar along the bottom edge whose colour
// follows the number (green, amber, terracotta); overloaded cards shake, dead ones grey out.
import { memo } from "react";
import { Handle, Position, useConnection, type Node as FlowNode, type NodeProps } from "@xyflow/react";
import { useTranslation } from "react-i18next";
import type { KindSpec, Node } from "@/bindings";
import { cn } from "@/lib/utils";
import { GlyphTile } from "@/solve/icons";
import { CARD_W } from "@/solve/store";
import { flavourName, kindName } from "@/solve/registry";
import { useTelemetry } from "@/solve/run/telemetry";

export type CardData = {
  node: Node;
  spec: KindSpec;
  /** Keyboard wiring: this card is the source, or the current target (ok or blocked). */
  wiring?: "source" | "target-ok" | "target-blocked";
  /** A still picture (a gallery thumbnail): never shows live numbers, which are keyed by
   * node id and belong to whichever design is running. */
  still?: boolean;
};
export type CardFlowNode = FlowNode<CardData, "component">;

const SIDES = [Position.Top, Position.Right, Position.Bottom, Position.Left];

export const ComponentCard = memo(function ComponentCard({ data, selected }: NodeProps<CardFlowNode>) {
  const { t } = useTranslation();
  const { node, spec, wiring } = data;
  const connecting = useConnection((c) => c.inProgress);
  const product = flavourName(t, node.flavour);
  const kind = kindName(t, node.kind);
  const title = node.label.trim() || product;
  const health = useTelemetry((s) => (data.still ? undefined : s.byId[node.id]));
  const stat = !health
    ? null
    : health.down
      ? t("canvas.statDown")
      : health.queue > 0
        ? t("canvas.statQueued", { util: health.util, queue: health.queue })
        : t("canvas.statBusy", { util: health.util });
  return (
    <div
      className={cn(
        "kb-card relative flex items-center gap-3 rounded-xl border bg-card px-3 py-2.5 text-card-foreground",
        selected ? "border-2 border-primary px-[11px] py-[9px]" : "border-border",
        wiring === "source" && "ring-2 ring-primary ring-offset-2 ring-offset-canvas",
        wiring === "target-ok" && "outline-2 outline-offset-4 outline-dashed outline-primary",
        wiring === "target-blocked" && "outline-2 outline-offset-4 outline-dashed outline-destructive",
        health?.overloaded && !health.down && "kb-overloaded",
        health?.down && "opacity-55 grayscale",
      )}
      style={{ width: CARD_W }}
      data-testid="component-card"
      data-kind={node.kind}
    >
      <GlyphTile icon={spec.icon} category={spec.category} className="size-9 [&_svg]:size-5" />
      <div className="min-w-0 flex-1 leading-tight">
        <div className="truncate font-semibold">{title}</div>
        <div className="truncate text-xs text-muted-foreground">
          {node.label.trim() ? `${product} · ${kind}` : kind}
        </div>
        {stat ? (
          <div className="truncate font-mono text-xs text-muted-foreground" data-testid="card-stat">
            {stat}
          </div>
        ) : (
          <div className="font-mono text-xs text-muted-foreground" aria-label={t("canvas.statPending")}>
            —
          </div>
        )}
      </div>
      {health && !health.down && (
        <div className="pointer-events-none absolute inset-x-3 bottom-0 h-[3px] overflow-hidden rounded-full" aria-hidden>
          <div
            className={cn(
              "h-full rounded-full transition-[width] duration-100",
              health.util >= 85 ? "bg-terracotta" : health.util >= 60 ? "bg-amber" : "bg-green",
            )}
            style={{ width: `${health.util}%` }}
          />
        </div>
      )}
      {SIDES.map((side) => (
        <Handle key={side} id={side} type="source" position={side} aria-label={t("canvas.connectFrom", { name: title })} />
      ))}
      {/* While a wire is being dragged, the whole card is a drop target. */}
      <Handle
        id="drop"
        type="target"
        position={Position.Top}
        className="kb-drop-handle"
        style={{
          position: "absolute",
          inset: 0,
          width: "100%",
          height: "100%",
          transform: "none",
          borderRadius: 12,
          border: 0,
          background: "transparent",
          opacity: 0,
          pointerEvents: connecting ? "all" : "none",
        }}
        isConnectableStart={false}
      />
    </div>
  );
});
