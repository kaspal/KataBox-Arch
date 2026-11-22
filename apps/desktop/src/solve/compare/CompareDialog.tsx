// Compare with reference (once solved): your design and the reference side by side, with
// linked zoom and pan, or overlaid, the reference drawn faintly behind yours and aligned by
// the centres of the two designs.
import { useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import type { Viewport } from "@xyflow/react";
import { Columns2, Layers } from "lucide-react";
import type { Design, Registry } from "@/bindings";
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group";
import { DesignView, designBounds, fitBox, type Box } from "@/solve/compare/DesignView";

type Mode = "split" | "overlay";

function useSize(ref: React.RefObject<HTMLElement | null>) {
  const [size, setSize] = useState({ w: 0, h: 0 });
  useLayoutEffect(() => {
    const el = ref.current;
    if (!el) return;
    const read = () => setSize({ w: el.clientWidth, h: el.clientHeight });
    read();
    const ro = new ResizeObserver(read);
    ro.observe(el);
    return () => ro.disconnect();
  }, [ref]);
  return size;
}

const centre = (b: Box | null) => (b ? { x: b.x + b.w / 2, y: b.y + b.h / 2 } : { x: 0, y: 0 });

function union(a: Box | null, b: Box | null): Box | null {
  if (!a || !b) return a ?? b;
  const x = Math.min(a.x, b.x);
  const y = Math.min(a.y, b.y);
  return { x, y, w: Math.max(a.x + a.w, b.x + b.w) - x, h: Math.max(a.y + a.h, b.y + b.h) - y };
}

export function CompareDialog({
  open,
  onClose,
  mine,
  reference,
  registry,
}: {
  open: boolean;
  onClose: () => void;
  mine: Design;
  reference: Design;
  registry: Registry;
}) {
  const { t } = useTranslation();
  const [mode, setMode] = useState<Mode>("split");
  return (
    <Dialog open={open} onOpenChange={(o) => !o && onClose()}>
      <DialogContent className="flex h-[85vh] max-w-[95vw] flex-col gap-3 sm:max-w-[95vw]" data-testid="compare">
        <DialogHeader className="flex-row items-center justify-between gap-4 pr-8">
          <div className="space-y-1">
            <DialogTitle>{t("compare.title")}</DialogTitle>
            <DialogDescription>{mode === "split" ? t("compare.splitHint") : t("compare.overlayHint")}</DialogDescription>
          </div>
          <ToggleGroup
            type="single"
            variant="outline"
            value={mode}
            onValueChange={(v) => v && setMode(v as Mode)}
            aria-label={t("compare.mode")}
          >
            <ToggleGroupItem value="split" className="gap-2 px-3">
              <Columns2 aria-hidden className="size-4" />
              {t("compare.split")}
            </ToggleGroupItem>
            <ToggleGroupItem value="overlay" className="gap-2 px-3">
              <Layers aria-hidden className="size-4" />
              {t("compare.overlay")}
            </ToggleGroupItem>
          </ToggleGroup>
        </DialogHeader>
        {open && (mode === "split" ? <Split mine={mine} reference={reference} registry={registry} /> : <Overlay mine={mine} reference={reference} registry={registry} />)}
      </DialogContent>
    </Dialog>
  );
}

function Pane({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <div className="flex min-h-0 min-w-0 flex-col gap-1">
      <h3 className="text-sm font-semibold">{title}</h3>
      <div className="relative min-h-0 flex-1 overflow-hidden rounded-md border bg-canvas">{children}</div>
    </div>
  );
}

type ViewsProps = { mine: Design; reference: Design; registry: Registry };

/** Two views with one viewport: zooming or panning either moves both. */
function Split({ mine, reference, registry }: ViewsProps) {
  const { t } = useTranslation();
  const box = useRef<HTMLDivElement>(null);
  const size = useSize(box);
  const bounds = useMemo(() => union(designBounds(mine), designBounds(reference)), [mine, reference]);
  const [vp, setVp] = useState<Viewport>({ x: 0, y: 0, zoom: 1 });
  const fitted = useRef(false);
  useEffect(() => {
    if (fitted.current || size.w === 0) return;
    fitted.current = true;
    // Each pane is half the width, less the gap.
    setVp(fitBox(bounds, { w: (size.w - 12) / 2, h: size.h - 24 }));
  }, [bounds, size]);
  return (
    <div ref={box} className="grid min-h-0 flex-1 grid-cols-2 gap-3">
      <Pane title={t("compare.yours")}>
        <DesignView design={mine} registry={registry} viewport={vp} onViewportChange={setVp} label={t("compare.yours")} testId="compare-mine" />
      </Pane>
      <Pane title={t("compare.reference")}>
        <DesignView design={reference} registry={registry} viewport={vp} onViewportChange={setVp} label={t("compare.reference")} testId="compare-reference" />
      </Pane>
    </div>
  );
}

/** Yours on top; the reference at 35% behind it, shifted so the two designs share a centre. */
function Overlay({ mine, reference, registry }: ViewsProps) {
  const { t } = useTranslation();
  const box = useRef<HTMLDivElement>(null);
  const size = useSize(box);
  const mineBox = useMemo(() => designBounds(mine), [mine]);
  const refBox = useMemo(() => designBounds(reference), [reference]);
  const shift = useMemo(() => {
    const a = centre(mineBox);
    const b = centre(refBox);
    return { x: a.x - b.x, y: a.y - b.y };
  }, [mineBox, refBox]);
  const [vp, setVp] = useState<Viewport>({ x: 0, y: 0, zoom: 1 });
  const fitted = useRef(false);
  useEffect(() => {
    if (fitted.current || size.w === 0) return;
    fitted.current = true;
    const shifted = refBox ? { ...refBox, x: refBox.x + shift.x, y: refBox.y + shift.y } : null;
    setVp(fitBox(union(mineBox, shifted), size));
  }, [mineBox, refBox, shift, size]);
  const ghost = { x: vp.x + shift.x * vp.zoom, y: vp.y + shift.y * vp.zoom, zoom: vp.zoom };
  return (
    <div className="flex min-h-0 flex-1 flex-col gap-1">
      <p className="flex gap-4 text-sm">
        <span className="font-semibold">{t("compare.yoursSolid")}</span>
        <span className="text-muted-foreground">{t("compare.referenceFaint")}</span>
      </p>
      <div ref={box} className="relative min-h-0 flex-1 overflow-hidden rounded-md border bg-canvas">
        <div className="pointer-events-none absolute inset-0 opacity-35" aria-hidden data-testid="compare-ghost">
          <DesignView design={reference} registry={registry} viewport={ghost} label={t("compare.reference")} interactive={false} />
        </div>
        <div className="absolute inset-0">
          <DesignView design={mine} registry={registry} viewport={vp} onViewportChange={setVp} label={t("compare.yours")} testId="compare-mine" />
        </div>
      </div>
    </div>
  );
}
