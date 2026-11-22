// /samples: the gallery. Each card shows its first design as a still picture (review A4: no
// engine runs here; the single live preview on Home arrives with the samples, M3.3).
import { useLayoutEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { Link } from "@tanstack/react-router";
import { PlayCircle } from "lucide-react";
import type { Design, Registry, SampleSummary } from "@/bindings";
import { EmptyState, Page } from "@/components/Page";
import { Skeleton } from "@/components/ui/skeleton";
import { describeError } from "@/lib/errors";
import { useRegistry, useSamples } from "@/lib/queries";
import { DesignView, designBounds, fitBox } from "@/solve/compare/DesignView";

/** A still picture of a design, fitted to its box. */
export function DesignThumb({ design, registry, label }: { design: Design; registry: Registry; label: string }) {
  const box = useRef<HTMLDivElement>(null);
  const [size, setSize] = useState({ w: 0, h: 0 });
  useLayoutEffect(() => {
    const el = box.current;
    if (!el) return;
    const read = () => setSize({ w: el.clientWidth, h: el.clientHeight });
    read();
    const ro = new ResizeObserver(read);
    ro.observe(el);
    return () => ro.disconnect();
  }, []);
  const fit = fitBox(designBounds(design), size, 16);
  const vp = { ...fit, zoom: Math.min(0.6, fit.zoom) };
  return (
    <div ref={box} className="pointer-events-none h-40 overflow-hidden rounded-md border bg-canvas" aria-hidden>
      {size.w > 0 && <DesignView design={design} registry={registry} viewport={vp} interactive={false} label={label} />}
    </div>
  );
}

function SampleCard({ s, registry }: { s: SampleSummary; registry: Registry }) {
  const { t, i18n } = useTranslation();
  const es = i18n.language.startsWith("es");
  const title = es ? s.title_es : s.title;
  return (
    <li>
      <Link
        to="/samples/$id"
        params={{ id: s.id }}
        className="flex h-full flex-col gap-3 rounded-xl border bg-card p-3 transition-colors hover:border-primary focus-visible:outline-2 focus-visible:outline-ring"
        data-testid="sample-card"
      >
        <DesignThumb design={s.preview} registry={registry} label={title} />
        <div className="space-y-1 px-1">
          <div className="flex items-baseline justify-between gap-2">
            <h2 className="font-semibold">{title}</h2>
            <span className="shrink-0 font-mono text-xs text-muted-foreground">{t("samples.steps", { count: s.steps })}</span>
          </div>
          <p className="text-sm text-muted-foreground">{es ? s.summary_es : s.summary}</p>
          {s.tour && <p className="text-xs font-semibold text-primary">{t("samples.tour")}</p>}
        </div>
      </Link>
    </li>
  );
}

export function SamplesPage() {
  const { t, i18n } = useTranslation();
  const samples = useSamples();
  const registry = useRegistry();
  return (
    <Page title={t("nav.samples")}>
      {samples.error ? (
        <EmptyState icon={PlayCircle} title={t("samples.unavailable")} body={describeError(i18n, samples.error).summary} />
      ) : !samples.data || !registry.data ? (
        <Skeleton className="h-60 w-full" />
      ) : samples.data.length === 0 ? (
        <EmptyState icon={PlayCircle} title={t("samples.none")} body={t("samples.noneBody")} />
      ) : (
        <ul className="grid gap-4 sm:grid-cols-2 xl:grid-cols-3">
          {samples.data.map((s) => (
            <SampleCard key={s.id} s={s} registry={registry.data} />
          ))}
        </ul>
      )}
    </Page>
  );
}
