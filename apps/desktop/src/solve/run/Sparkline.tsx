// A small uPlot line chart (p95 per simulated second in the Metrics tab). Colours come from
// the design tokens; the numbers are always also shown as text next to it.
import { useEffect, useRef } from "react";
import uPlot from "uplot";
import "uplot/dist/uPlot.min.css";

const canvasWorks = () => {
  try {
    return !!document.createElement("canvas").getContext("2d");
  } catch {
    return false;
  }
};

export function Sparkline({
  xs,
  ys,
  label,
  height = 56,
  testId,
}: {
  xs: number[];
  ys: number[];
  /** What the line shows (the chart is decorative next to the number; this names it). */
  label: string;
  height?: number;
  testId?: string;
}) {
  const box = useRef<HTMLDivElement>(null);
  const plot = useRef<uPlot | null>(null);

  useEffect(() => {
    const el = box.current;
    if (!el || !canvasWorks()) return;
    const css = getComputedStyle(document.documentElement);
    const stroke = css.getPropertyValue("--primary").trim() || "#3b6ea8";
    const u = new uPlot(
      {
        width: el.clientWidth || 240,
        height,
        legend: { show: false },
        cursor: { show: false },
        scales: { x: { time: false } },
        axes: [{ show: false }, { show: false }],
        series: [{}, { stroke, width: 1.5, points: { show: false } }],
      },
      [[], []],
      el,
    );
    plot.current = u;
    const ro = new ResizeObserver(() => u.setSize({ width: el.clientWidth || 240, height }));
    ro.observe(el);
    return () => {
      ro.disconnect();
      u.destroy();
      plot.current = null;
    };
  }, [height]);

  useEffect(() => {
    plot.current?.setData([xs, ys]);
  }, [xs, ys]);

  return <div ref={box} role="img" aria-label={label} className="w-full" style={{ height }} data-testid={testId} />;
}
