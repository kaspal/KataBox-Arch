// Region and AZ frames: a dashed border drawn behind the cards. Only the label is a drag
// handle (the inside lets clicks through to the canvas and the cards); resizable when
// selected.
import { memo } from "react";
import { NodeResizer, type Node as FlowNode, type NodeProps } from "@xyflow/react";
import { useTranslation } from "react-i18next";
import type { Group } from "@/bindings";
import { cn } from "@/lib/utils";
import { useEditor } from "@/solve/store";

export type ZoneData = { group: Group };
export type ZoneFlowNode = FlowNode<ZoneData, "zone">;

export const ZoneNode = memo(function ZoneNode({ data, selected }: NodeProps<ZoneFlowNode>) {
  const { t } = useTranslation();
  const { group } = data;
  const beginDrag = useEditor((s) => s.beginDrag);
  const endDrag = useEditor((s) => s.endDrag);
  const kindLabel = t(`canvas.zone.${group.kind}`);
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
          "h-full w-full rounded-xl border-[1.5px] border-dashed",
          selected ? "border-primary" : "border-[var(--zone-line)]",
          group.kind === "region" ? "bg-[color-mix(in_srgb,var(--card)_35%,transparent)]" : "",
        )}
        data-testid="zone"
      >
        <div className="kb-zone-handle inline-flex cursor-grab items-center gap-1.5 rounded-br-lg rounded-tl-xl px-3 py-1.5 font-mono text-xs text-muted-foreground">
          <span className="font-semibold text-foreground">{group.label.trim() || kindLabel}</span>
          {group.label.trim() && <span>· {kindLabel}</span>}
        </div>
      </div>
    </>
  );
});
