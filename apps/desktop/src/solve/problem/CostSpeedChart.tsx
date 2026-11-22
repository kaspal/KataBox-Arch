// Cost vs speed: every accepted submission (monthly cost across, first-test p95 up) and the
// reference as a star. Lower and further left is better. The chart is a picture of numbers
// that are also written out under it, so nothing depends on colour or on the canvas.
import { useEffect, useMemo, useRef } from "react";
import { useTranslation } from "react-i18next";
import uPlot from "uplot";
import "uplot/dist/uPlot.min.css";
import type { ReferencePoint, SubmissionSummary } from "@/bindings";

const canvasWorks = () => {
  try {
    return !!document.createElement("canvas").getContext("2d");
  } catch {
    return false;
  }
};

const token = (name: string, fallback: string) =>
  getComputedStyle(document.documentElement).getPropertyValue(name).trim() || fallback;

function star(ctx: CanvasRenderingContext2D, x: number, y: number, r: number) {
  ctx.beginPath();
  for (let i = 0; i < 10; i++) {
    const a = -Math.PI / 2 + (i * Math.PI) / 5;
    const rr = i % 2 === 0 ? r : r * 0.45;
    ctx.lineTo(x + rr * Math.cos(a), y + rr * Math.sin(a));
  }
  ctx.closePath();
}

export function CostSpeedChart({
  submissions,
  reference,
  height = 180,
}: {
  submissions: SubmissionSummary[];
  reference: ReferencePoint | null;
  height?: number;
}) {
  const { t, i18n } = useTranslation();
  const box = useRef<HTMLDivElement>(null);
  const accepted = useMemo(() => submissions.filter((s) => s.verdict === "accepted"), [submissions]);
  const usd = useMemo(() => new Intl.NumberFormat(i18n.language, { style: "currency", currency: "USD", maximumFractionDigits: 0 }), [i18n.language]);
  const ms = useMemo(() => new Intl.NumberFormat(i18n.language, { maximumFractionDigits: 1 }), [i18n.language]);
  const best = accepted.reduce<SubmissionSummary | null>((b, s) => (!b || s.cost_usd < b.cost_usd ? s : b), null);

  useEffect(() => {
    const el = box.current;
    if (!el || !canvasWorks() || (accepted.length === 0 && !reference)) return;
    // uPlot wants one sorted x array; each series has a value or a gap at every x.
    const pts = [
      ...accepted.map((s) => ({ x: s.cost_usd, y: s.p95_ms, ref: false })),
      ...(reference ? [{ x: reference.cost_usd, y: reference.p95_ms, ref: true }] : []),
    ].sort((a, b) => a.x - b.x);
    const xs = pts.map((p) => p.x);
    const mineYs = pts.map((p) => (p.ref ? null : p.y));
    const refYs = pts.map((p) => (p.ref ? p.y : null));
    const primary = token("--primary", "#3b6ea8");
    const accent = token("--terracotta", "#c2410c");
    const axis = token("--muted-foreground", "#6b7280");
    const grid = token("--border", "#e5e7eb");
    const font = "11px ui-sans-serif, system-ui";
    const pad = (min: number, max: number): [number, number] => {
      const span = max - min || Math.max(1, max * 0.2);
      return [Math.max(0, min - span * 0.15), max + span * 0.15];
    };
    const u = new uPlot(
      {
        width: el.clientWidth || 300,
        height,
        legend: { show: false },
        cursor: { show: false },
        scales: {
          x: { time: false, auto: false, range: pad(Math.min(...xs), Math.max(...xs)) },
          y: { auto: false, range: pad(Math.min(...pts.map((p) => p.y)), Math.max(...pts.map((p) => p.y))) },
        },
        axes: [
          { stroke: axis, grid: { stroke: grid, width: 1 }, ticks: { stroke: grid }, font, values: (_u, v) => v.map((x) => usd.format(x)) },
          { stroke: axis, grid: { stroke: grid, width: 1 }, ticks: { stroke: grid }, font, size: 44, values: (_u, v) => v.map((y) => `${ms.format(y)}`) },
        ],
        series: [
          {},
          { stroke: primary, fill: primary, paths: () => null, points: { show: true, size: 8, fill: primary, stroke: primary } },
          // The reference keeps the scales honest; its star is drawn below.
          { stroke: accent, paths: () => null, points: { show: false } },
        ],
        hooks: {
          draw: [
            (u) => {
              const i = refYs.findIndex((y) => y !== null);
              if (i < 0) return;
              const x = u.valToPos(xs[i], "x", true);
              const y = u.valToPos(refYs[i] as number, "y", true);
              const ctx = u.ctx;
              ctx.save();
              // An outline, so a submission at the same point still shows inside it.
              star(ctx, x, y, 10 * uPlot.pxRatio);
              ctx.strokeStyle = accent;
              ctx.lineWidth = 2 * uPlot.pxRatio;
              ctx.lineJoin = "round";
              ctx.stroke();
              ctx.restore();
            },
          ],
        },
      },
      [xs, mineYs, refYs] as uPlot.AlignedData,
      el,
    );
    const ro = new ResizeObserver(() => u.setSize({ width: el.clientWidth || 300, height }));
    ro.observe(el);
    return () => {
      ro.disconnect();
      u.destroy();
    };
  }, [accepted, reference, height, usd, ms]);

  if (accepted.length === 0 && !reference) {
    return <p className="text-sm text-muted-foreground">{t("chart.empty")}</p>;
  }
  return (
    <figure className="space-y-2" data-testid="cost-speed">
      <div className="flex flex-wrap gap-x-4 gap-y-1 text-xs">
        <span className="flex items-center gap-1.5">
          <span aria-hidden className="inline-block size-2.5 rounded-full bg-primary" />
          {t("chart.yours")}
        </span>
        {reference && (
          <span className="flex items-center gap-1.5">
            <span aria-hidden className="text-[var(--terracotta-ink)]">☆</span>
            {t("chart.reference")}
          </span>
        )}
      </div>
      <div ref={box} role="img" aria-label={t("chart.label")} className="w-full" style={{ height }} />
      <figcaption className="space-y-0.5 text-xs text-muted-foreground">
        <div>{t("chart.axes")}</div>
        {best && <div data-testid="chart-best">{t("chart.best", { cost: usd.format(best.cost_usd), p95: ms.format(best.p95_ms), n: accepted.length })}</div>}
        {reference && <div data-testid="chart-reference">{t("chart.referenceAt", { cost: usd.format(reference.cost_usd), p95: ms.format(reference.p95_ms) })}</div>}
      </figcaption>
    </figure>
  );
}
