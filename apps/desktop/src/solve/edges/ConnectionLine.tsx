// The wire being dragged: the same curve shape as a real edge, and when the card under
// the pointer can't take it, the reason in a tooltip next to the pointer.
import { useStore, useViewport, type ConnectionLineComponentProps } from "@xyflow/react";
import { useTranslation } from "react-i18next";
import { routeEdge, toPath } from "@/solve/geometry/routeEdge";
import { useCanvas } from "@/solve/context";
import { useEditor } from "@/solve/store";
import { checkWire } from "@/solve/wiring";
import { nodeRect } from "@/solve/edges/CurvedEdge";

export function ConnectionLine({ fromNode, toNode, toX, toY }: ConnectionLineComponentProps) {
  const { t } = useTranslation();
  const { wiring } = useCanvas();
  const doc = useEditor((s) => s.doc);
  // The line is drawn in canvas coordinates; the tooltip stays at screen size.
  const { x: panX, zoom } = useViewport();
  const paneWidth = useStore((s) => s.width);
  // Near the right edge, the tooltip opens to the pointer's left.
  const flip = toX * zoom + panX + 290 > paneWidth;
  const tipX = flip ? toX - 274 / zoom : toX + 14 / zoom;
  const from = nodeRect(fromNode);
  const pointer = { x: toX - 1, y: toY - 1, w: 2, h: 2 };
  const target = toNode && toNode.id !== fromNode.id ? nodeRect(toNode) : pointer;
  const route = routeEdge(from, target);
  const check = toNode ? checkWire(wiring, doc.nodes, doc.edges, fromNode.id, toNode.id) : null;
  const blocked = check && !check.ok ? t(`wiring.reason.${check.reason}`) : null;
  return (
    <g>
      <path
        d={toPath({ ...route, p3: toNode ? route.p3 : { x: toX, y: toY } })}
        fill="none"
        stroke={blocked ? "var(--destructive)" : "var(--primary)"}
        strokeWidth={1.75}
        strokeDasharray={blocked ? "4 4" : undefined}
      />
      {blocked && (
        <foreignObject x={tipX} y={toY + 10 / zoom} width={1} height={1} style={{ overflow: "visible" }}>
          <div
            style={{ transform: `scale(${1 / zoom})`, transformOrigin: "0 0", width: 260 }}
            role="tooltip"
            className="rounded-md bg-foreground px-2.5 py-1.5 text-xs text-background shadow"
            data-testid="wiring-blocked"
          >
            {blocked}
          </div>
        </foreignObject>
      )}
    </g>
  );
}
