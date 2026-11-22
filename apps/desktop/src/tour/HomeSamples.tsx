// Home's "Watch how it works": the samples, each with a picture of its first design. One
// preview at a time is live (review A4): the card under the pointer or keyboard focus, or
// else the first card while the gallery is on screen. It plays the sample's first traffic
// for 6 seconds, then starts over. The others are still pictures. Nothing animates with
// reduced motion, and the live preview stops while the window is hidden.
import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { Link } from "@tanstack/react-router";
import type { Registry, SampleSummary, SandboxPreset } from "@/bindings";
import { Skeleton } from "@/components/ui/skeleton";
import { useReducedMotion } from "@/lib/motion";
import { useRegistry, useSamples } from "@/lib/queries";
import { DesignView, designBounds, fitBox } from "@/solve/compare/DesignView";
import { PacketLayer } from "@/solve/run/PacketLayer";
import { useRun } from "@/solve/run/runStore";
import { startTelemetry } from "@/solve/run/telemetry";
import { DesignThumb } from "@/tour/SamplesPage";

/** How long a preview loop lasts (wall-clock ms). */
export const PREVIEW_MS = 6000;

function useInView(ref: React.RefObject<HTMLElement | null>) {
  const [inView, setInView] = useState(false);
  useEffect(() => {
    const el = ref.current;
    if (!el || typeof IntersectionObserver === "undefined") return;
    const io = new IntersectionObserver(([e]) => setInView(e.isIntersecting), { threshold: 0.3 });
    io.observe(el);
    return () => io.disconnect();
  }, [ref]);
  return inView;
}

function useWindowVisible() {
  const [visible, setVisible] = useState(() => document.visibilityState !== "hidden");
  useEffect(() => {
    const on = () => setVisible(document.visibilityState !== "hidden");
    document.addEventListener("visibilitychange", on);
    return () => document.removeEventListener("visibilitychange", on);
  }, []);
  return visible;
}

/** The live preview: the first design playing its first traffic, looped. */
function LivePreview({ s, registry }: { s: SampleSummary; registry: Registry }) {
  const box = useRef<HTMLDivElement>(null);
  const [size, setSize] = useState({ w: 0, h: 0 });
  useLayoutEffect(() => {
    const el = box.current;
    if (el) setSize({ w: el.clientWidth, h: el.clientHeight });
  }, []);
  useEffect(() => {
    const saved = useRun.getState();
    const { preset, problemTest, speed } = saved;
    const run = s.preview_run;
    const stopTelemetry = startTelemetry();
    const play = () => {
      if (!run) return;
      useRun.setState({
        preset: (run.preset ?? "normal") as SandboxPreset,
        problemTest: run.test && s.problem ? { problem_id: s.problem, test_id: run.test } : null,
        speed: run.speed,
      });
      useRun.getState().start(s.preview, { startS: run.start_s }).catch(() => {});
    };
    play();
    const loop = window.setInterval(play, PREVIEW_MS);
    return () => {
      window.clearInterval(loop);
      stopTelemetry();
      useRun.getState().stop();
      useRun.setState({ preset, problemTest, speed });
    };
  }, [s]);
  const fit = fitBox(designBounds(s.preview), size, 12, 0.12);
  return (
    <div ref={box} className="pointer-events-none h-40 overflow-hidden rounded-md border bg-canvas" aria-hidden data-testid="live-preview">
      {size.w > 0 && (
        <DesignView design={s.preview} registry={registry} viewport={{ ...fit, zoom: Math.min(0.6, fit.zoom) }} interactive={false} label={s.title}>
          <PacketLayer />
        </DesignView>
      )}
    </div>
  );
}

export function HomeSamples() {
  const { t, i18n } = useTranslation();
  const samples = useSamples();
  const registry = useRegistry();
  const section = useRef<HTMLElement>(null);
  const inView = useInView(section);
  const visible = useWindowVisible();
  const reduced = useReducedMotion();
  const [pointed, setPointed] = useState<string | null>(null);
  const es = i18n.language.startsWith("es");
  const list = samples.data ?? [];
  if (samples.error || (samples.data && list.length === 0)) return null;
  const live = reduced || !visible ? null : (pointed ?? (inView ? (list[0]?.id ?? null) : null));
  return (
    <section ref={section} aria-labelledby="samples-title" className="space-y-4" data-testid="home-samples">
      <div className="flex items-baseline justify-between">
        <h2 id="samples-title" className="font-display text-3xl font-semibold">
          {t("home.watch")}
        </h2>
        <Link to="/samples" className="text-sm font-semibold text-primary underline underline-offset-4">
          {t("home.allSamples")}
        </Link>
      </div>
      {!samples.data || !registry.data ? (
        <Skeleton className="h-56 w-full" />
      ) : (
        <ul className="grid grid-cols-[repeat(auto-fill,minmax(200px,1fr))] gap-4">
          {list.map((s) => {
            const title = es ? s.title_es : s.title;
            return (
              <li key={s.id} onMouseEnter={() => setPointed(s.id)} onMouseLeave={() => setPointed((p) => (p === s.id ? null : p))}>
                <Link
                  to="/samples/$id"
                  params={{ id: s.id }}
                  className="flex h-full flex-col gap-3 rounded-xl border bg-card p-3 transition-colors hover:border-primary focus-visible:outline-2 focus-visible:outline-ring"
                  onFocus={() => setPointed(s.id)}
                  onBlur={() => setPointed((p) => (p === s.id ? null : p))}
                  data-testid="home-sample"
                >
                  {live === s.id ? <LivePreview s={s} registry={registry.data} /> : <DesignThumb design={s.preview} registry={registry.data} label={title} />}
                  <div className="space-y-1 px-1">
                    <div className="flex items-baseline justify-between gap-2">
                      <h3 className="font-semibold leading-snug">{title}</h3>
                      <span className="shrink-0 font-mono text-xs text-muted-foreground">{t("samples.steps", { count: s.steps })}</span>
                    </div>
                    <p className="text-sm text-muted-foreground">{es ? s.summary_es : s.summary}</p>
                  </div>
                </Link>
              </li>
            );
          })}
        </ul>
      )}
    </section>
  );
}
