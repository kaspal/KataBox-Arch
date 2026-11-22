// The walkthrough player (plan M3.2), shared by the samples, the first-run tour and (M3.5)
// the concept cards. A story is a list of steps; each step shows one design snapshot,
// moves the camera to it (eased; a cut with reduced motion), dims what isn't in focus,
// shows a caption card in the hand-written face, and may play traffic and call out a number.
//
// - Traffic goes through the same live run as the editor (one run at a time; the player owns
//   it while open and stops it on the way out).
// - Back / Next / the step dots move between steps; "Replay step" plays the step again;
//   "Try it yourself" hands the step's design to the caller (the Sandbox forks it).
import { useCallback, useEffect, useLayoutEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import type { Viewport } from "@xyflow/react";
import { ChevronLeft, ChevronRight, LayoutGrid, RotateCcw } from "lucide-react";
import type { CalloutMetric, Design, Registry, SandboxPreset, StoryStep } from "@/bindings";
import { useReportError } from "@/app/dialogs";
import { Button } from "@/components/ui/button";
import { useReducedMotion } from "@/lib/motion";
import { cn } from "@/lib/utils";
import { DesignView, designBounds, fitBox, type Box } from "@/solve/compare/DesignView";
import { PacketLayer } from "@/solve/run/PacketLayer";
import { useRun } from "@/solve/run/runStore";
import { startTelemetry } from "@/solve/run/telemetry";
import { CARD_H, CARD_W } from "@/solve/store";

export type Story = {
  title: string;
  /** The problem whose tests `run.test` names, if any. */
  problemId: string | null;
  steps: StoryStep[];
  snapshots: Partial<Record<string, Design>>;
};

/** How long a camera move takes (ms). */
export const PAN_MS = 700;

/** The app's out easing (cubic-bezier(.23,1,.32,1)), close enough as an out-quint. */
const ease = (t: number) => 1 - Math.pow(1 - t, 5);

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

/** The caption card's band along the bottom of the player. */
export const CAPTION_H = 160;
/** Space kept between the focused cards and the edge of the view. */
export const FRAME_MARGIN = 48;

/** The box a step must show: its focused cards, else the whole design. */
export function focusBox(step: StoryStep, design: Design): Box | null {
  const focus = step.focus ?? [];
  const lit = design.nodes.filter((n) => focus.includes(n.id));
  if (lit.length === 0) return designBounds(design);
  const x0 = Math.min(...lit.map((n) => n.position.x));
  const y0 = Math.min(...lit.map((n) => n.position.y));
  const x1 = Math.max(...lit.map((n) => n.position.x + CARD_W));
  const y1 = Math.max(...lit.map((n) => n.position.y + CARD_H));
  return { x: x0, y: y0, w: x1 - x0, h: y1 - y0 };
}

/**
 * Where a step's camera looks, above the caption band. The focused cards (else the design)
 * always fit with a margin (M1–M3 review F2): a step's own camera keeps its centre and zoom
 * only as far as they still show every focused card; otherwise it zooms out and slides just
 * enough.
 */
export function stepViewport(step: StoryStep, design: Design, size: { w: number; h: number }): Viewport {
  const area = { w: size.w, h: Math.max(1, size.h - CAPTION_H) };
  const box = focusBox(step, design);
  const fit = fitBox(box, area, FRAME_MARGIN, 0.05);
  const fitted = { ...fit, zoom: Math.min(fit.zoom, 1.2) };
  if (!step.camera || !box) return fitted;
  const zoom = Math.min(step.camera.zoom, fitted.zoom);
  let x = area.w / 2 - step.camera.x * zoom;
  let y = area.h / 2 - step.camera.y * zoom;
  const clamp = (pos: number, lo: number, hi: number, room: number) => {
    // Slide so [lo, hi] (screen coordinates) sits inside [margin, room - margin].
    if (lo + pos < FRAME_MARGIN) return FRAME_MARGIN - lo;
    if (hi + pos > room - FRAME_MARGIN) return room - FRAME_MARGIN - hi;
    return pos;
  };
  x = clamp(x, box.x * zoom, (box.x + box.w) * zoom, area.w);
  y = clamp(y, box.y * zoom, (box.y + box.h) * zoom, area.h);
  return { x, y, zoom };
}

function Callout({ step }: { step: StoryStep }) {
  const { t, i18n } = useTranslation();
  const c = step.callout;
  if (!c) return null;
  const fmt = (m: CalloutMetric, v: number) => {
    const n = (o: Intl.NumberFormatOptions) => new Intl.NumberFormat(i18n.language, o).format(v);
    switch (m) {
      case "p95_ms":
        return t("player.ms", { v: n({ maximumFractionDigits: v < 10 ? 1 : 0 }) });
      case "error_rate":
        return n({ style: "percent", maximumFractionDigits: 1 });
      case "throughput_rps":
        return t("player.rps", { v: n({ maximumFractionDigits: 0 }) });
      case "cost_month":
        return n({ style: "currency", currency: "USD", maximumFractionDigits: 0 });
      case "lost":
        return n({ maximumFractionDigits: 0 });
    }
  };
  return (
    <aside className="absolute right-4 top-4 w-56 rounded-xl border bg-card p-4 shadow-sm" data-testid="callout">
      <div className="text-sm text-muted-foreground">{t(`player.metric.${c.metric}`)}</div>
      {c.before !== null && c.before !== undefined && (
        <div className="font-mono text-sm text-muted-foreground line-through" aria-label={t("player.before", { v: fmt(c.metric, c.before) })}>
          {fmt(c.metric, c.before)}
        </div>
      )}
      <div className="font-mono text-3xl tabular-nums">{fmt(c.metric, c.after)}</div>
      {c.target !== null && c.target !== undefined && (
        <div className="mt-1 text-xs text-muted-foreground">{t("player.target", { v: fmt(c.metric, c.target) })}</div>
      )}
    </aside>
  );
}

export function Player({
  story,
  registry,
  onTry,
  header,
}: {
  story: Story;
  registry: Registry;
  /** "Try it yourself" with the step's design. */
  onTry?: (design: Design) => void;
  /** Left of the step counter (a back link and the title). */
  header?: React.ReactNode;
}) {
  const { t, i18n } = useTranslation();
  const report = useReportError();
  const reduced = useReducedMotion();
  const es = i18n.language.startsWith("es");
  const [index, setIndex] = useState(0);
  const [replays, setReplays] = useState(0);
  const step = story.steps[index];
  const design = story.snapshots[step.snapshot];
  const pane = useRef<HTMLDivElement>(null);
  const size = useSize(pane);
  const [vp, setVp] = useState<Viewport>({ x: 0, y: 0, zoom: 1 });
  const vpRef = useRef(vp);
  const setView = useCallback((v: Viewport) => {
    vpRef.current = v;
    setVp(v);
  }, []);
  const last = story.steps.length - 1;
  const title = (s: StoryStep) => (es ? s.title_es : s.title);

  // Live telemetry for the cards while the player is open. On the way out the run stops and
  // the run settings the steps changed (preset, test, speed) go back to what they were.
  useEffect(() => {
    const { preset, problemTest, speed } = useRun.getState();
    const stopTelemetry = startTelemetry();
    return () => {
      stopTelemetry();
      useRun.getState().stop();
      useRun.setState({ preset, problemTest, speed });
    };
  }, []);

  // The camera: ease from where it is to the step's view (a cut with reduced motion).
  useEffect(() => {
    if (!design || size.w === 0) return;
    const to = stepViewport(step, design, size);
    if (reduced) {
      setView(to);
      return;
    }
    const from = vpRef.current;
    const t0 = performance.now();
    let raf = 0;
    const tick = (now: number) => {
      const k = ease(Math.min(1, (now - t0) / PAN_MS));
      setView({ x: from.x + (to.x - from.x) * k, y: from.y + (to.y - from.y) * k, zoom: from.zoom + (to.zoom - from.zoom) * k });
      if (k < 1) raf = requestAnimationFrame(tick);
    };
    raf = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(raf);
    // `replays` moves the camera back too.
  }, [step, design, size, reduced, replays, setView]);

  // Traffic: each step (or replay) starts its own run, or stops the last one.
  useEffect(() => {
    const run = useRun.getState();
    if (!design || !step.run) {
      run.stop();
      return;
    }
    const r = step.run;
    useRun.setState({
      preset: (r.preset ?? "normal") as SandboxPreset,
      problemTest: r.test && story.problemId ? { problem_id: story.problemId, test_id: r.test } : null,
      speed: r.speed,
    });
    run.start(design, { startS: r.start_s }).catch(report);
  }, [step, design, story.problemId, replays, report]);

  const go = useCallback((i: number) => setIndex(Math.max(0, Math.min(last, i))), [last]);

  // Arrow keys step through (unless typing).
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.target instanceof HTMLElement && ["INPUT", "TEXTAREA", "SELECT"].includes(e.target.tagName)) return;
      if (e.key === "ArrowRight") go(index + 1);
      else if (e.key === "ArrowLeft") go(index - 1);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [go, index]);

  const caption = es ? step.caption_es : step.caption;
  const next = index < last ? story.steps[index + 1] : null;
  const dots = story.steps.map((s, i) => ({ i, label: t("player.goTo", { n: i + 1, title: title(s) }) }));

  return (
    <div className="flex h-full min-h-0 flex-col" data-testid="player">
      <header className="flex items-center justify-between gap-4 border-b px-4 py-3">
        <div className="flex min-w-0 items-center gap-2">{header}</div>
        <div className="shrink-0 font-mono text-xs text-muted-foreground" aria-live="polite" data-testid="step-counter">
          {t("player.stepOf", { n: index + 1, total: story.steps.length })}
        </div>
      </header>
      <div ref={pane} className="relative min-h-0 flex-1 bg-canvas">
        {design && (
          <DesignView design={design} registry={registry} viewport={vp} onViewportChange={setView} focus={step.focus} label={title(step)} testId="player-canvas">
            <PacketLayer />
          </DesignView>
        )}
        <Callout step={step} />
        <section
          className="absolute bottom-4 left-4 max-w-lg rounded-xl border bg-card p-4 shadow-sm"
          aria-labelledby="player-step-title"
          data-testid="caption"
        >
          <h2 id="player-step-title" className="font-hand text-xl leading-snug">
            {t("player.stepTitle", { n: index + 1, title: title(step) })}
          </h2>
          <p className="mt-1 font-hand text-base leading-relaxed">{caption}</p>
        </section>
      </div>
      <footer className="flex flex-wrap items-center justify-between gap-3 border-t px-4 py-3">
        <div className="flex gap-2">
          <Button variant="outline" className="gap-1" disabled={index === 0} onClick={() => go(index - 1)}>
            <ChevronLeft aria-hidden className="size-4" />
            {t("player.previous")}
          </Button>
          <Button className="gap-1" disabled={!next} onClick={() => go(index + 1)}>
            <ChevronRight aria-hidden className="size-4" />
            {next ? t("player.nextTo", { title: title(next) }) : t("player.end")}
          </Button>
        </div>
        <ol className="flex items-center gap-1.5" aria-label={t("player.steps")}>
          {dots.map((d) => (
            <li key={d.i}>
              <button
                type="button"
                className={cn("block h-2 rounded-full bg-muted-foreground/40 transition-all", d.i === index ? "w-5 bg-primary" : "w-2")}
                aria-label={d.label}
                aria-current={d.i === index ? "step" : undefined}
                onClick={() => go(d.i)}
              />
            </li>
          ))}
        </ol>
        <div className="flex gap-2">
          <Button variant="outline" className="gap-2" onClick={() => setReplays((n) => n + 1)}>
            <RotateCcw aria-hidden className="size-4" />
            {t("player.replay")}
          </Button>
          {onTry && design && (
            <Button variant="outline" className="gap-2" onClick={() => onTry(design)}>
              <LayoutGrid aria-hidden className="size-4" />
              {t("player.try")}
            </Button>
          )}
        </div>
      </footer>
    </div>
  );
}
