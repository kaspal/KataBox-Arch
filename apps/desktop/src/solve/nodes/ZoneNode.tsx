// Zone frames drawn behind the cards: Region and AZ (dashed), and Autoscaling groups
// (a compute-tinted frame showing the instance range). Only the label is a drag handle
// (the inside lets clicks through to the canvas and the cards); resizable when selected.
import { memo } from "react";
import { NodeResizer, type Node as FlowNode, type NodeProps } from "@xyflow/react";
import { useTranslation } from "react-i18next";
import type { Group } from "@/bindings";
import { cn } from "@/lib/utils";
import { Glyph } from "@/solve/icons";
import { useCanvas } from "@/solve/context";
import { flavourName, kindSpec } from "@/solve/registry";
import { useEditor } from "@/solve/store";

export type ZoneData = { group: Group };
export type ZoneFlowNode = FlowNode<ZoneData, "zone">;

export const ZoneNode = memo(function ZoneNode({ data, selected }: NodeProps<ZoneFlowNode>) {
  const { t } = useTranslation();
  const { group } = data;
  const beginDrag = useEditor((s) => s.beginDrag);
  const endDrag = useEditor((s) => s.endDrag);
  const kindLabel = t(`canvas.zone.${group.kind}`);
  const { registry } = useCanvas();
  const scaling = group.kind === "autoscale" ? group.scaling : null;
  // Props missing from the file take the Autoscaling kind's defaults, as in the simulator.
  const prop = (key: string) => {
    const v = scaling?.props[key] ?? kindSpec(registry, "autoscale").props.find((p) => p.key === key)?.default;
    return typeof v === "number" ? v : "?";
  };
  const range = scaling ? `${prop("min_instances")}–${prop("max_instances")} · ${flavourName(t, scaling.flavour)}` : null;
  return (
    <>
      <NodeResizer
        isVisible={selected}
        minWidth={160}
        minHeight={120}
        lineClassName="kb-zone-resize"
        handleClassName="kb-zone-resize"
        // Live sizes arrive through the canvas's onNodesChange; one resize is one undo step.
        onResizeStart={() => beginDrag()}
        onResizeEnd={() => endDrag()}
      />
      <div
        className={cn(
          "h-full w-full rounded-xl border-[1.5px]",
          scaling ? "border-solid" : "border-dashed",
          selected ? "border-primary" : scaling ? "border-[var(--cat-compute)]" : "border-[var(--zone-line)]",
          group.kind === "region" && "bg-[color-mix(in_srgb,var(--card)_35%,transparent)]",
          scaling && "bg-[color-mix(in_srgb,var(--cat-compute)_6%,transparent)]",
        )}
        data-testid="zone"
      >
        <div className="kb-zone-handle inline-flex cursor-grab items-center gap-1.5 rounded-br-lg rounded-tl-xl px-3 py-1.5 font-mono text-xs text-muted-foreground">
          {scaling && (
            <span style={{ color: "var(--cat-compute-glyph)" }}>
              <Glyph icon="autoscale" className="size-3.5" />
            </span>
          )}
          <span className="font-semibold text-foreground">{group.label.trim() || kindLabel}</span>
          {group.label.trim() && <span>· {kindLabel}</span>}
          {range && <span>· {range}</span>}
        </div>
      </div>
    </>
  );
});
