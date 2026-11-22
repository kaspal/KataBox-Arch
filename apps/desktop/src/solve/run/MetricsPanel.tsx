// The Metrics tab: live tiles (p95, throughput, error rate, cost) and the busiest cards.
// Every value is text; colour only reinforces it (CLAUDE.md).
import { useTranslation } from "react-i18next";
import { flavourName } from "@/solve/registry";
import { useEditor } from "@/solve/store";
import { NODE_DOWN } from "@/solve/run/frames";
import { useRun } from "@/solve/run/runStore";
import { Sparkline } from "@/solve/run/Sparkline";

function Tile({ label, value, unit, testId }: { label: string; value: string; unit?: string; testId: string }) {
  return (
    <div className="min-w-0 rounded-md border bg-card p-3">
      <div className="truncate text-xs text-muted-foreground">{label}</div>
      <div className="mt-1 flex flex-wrap items-baseline gap-x-1 font-mono tabular-nums" data-testid={testId}>
        <span className="text-lg">{value}</span>
        {unit && <span className="text-xs text-muted-foreground">{unit}</span>}
      </div>
    </div>
  );
}

export function MetricsPanel() {
  const { t, i18n } = useTranslation();
  const frame = useRun((s) => s.frame);
  const info = useRun((s) => s.info);
  const series = useRun((s) => s.series);
  const nodes = useEditor((s) => s.doc.nodes);
  if (!frame) {
    return <p className="text-sm text-muted-foreground">{t("metrics.empty")}</p>;
  }
  const n0 = new Intl.NumberFormat(i18n.language, { maximumFractionDigits: 0 });
  const n1 = new Intl.NumberFormat(i18n.language, { maximumFractionDigits: 1 });
  const pct = new Intl.NumberFormat(i18n.language, { style: "percent", maximumFractionDigits: 2 });
  const usd = new Intl.NumberFormat(i18n.language, { style: "currency", currency: "USD", maximumFractionDigits: 0 });

  const byId = new Map(nodes.map((n) => [n.id, n]));
  const busiest = (info?.nodes ?? [])
    .map((id, i) => ({ id, util: frame.util[i] ?? 0, down: ((frame.nodeFlags[i] ?? 0) & NODE_DOWN) !== 0 }))
    .filter((x) => x.util > 0 || x.down)
    .sort((a, b) => b.util - a.util || a.id.localeCompare(b.id))
    .slice(0, 4);

  return (
    <div className="space-y-4" data-testid="metrics">
      <div className="grid grid-cols-2 gap-2">
        <Tile label={t("metrics.p95")} value={n1.format(frame.p95Ms)} unit={t("units.ms")} testId="metric-p95" />
        <Tile label={t("metrics.throughput")} value={n0.format(frame.rps)} unit={t("units.rps")} testId="metric-rps" />
        <Tile label={t("metrics.errorRate")} value={pct.format(frame.errorRate)} testId="metric-errors" />
        <Tile label={t("metrics.cost")} value={usd.format(frame.costMonth)} unit={t("metrics.perMonth")} testId="metric-cost" />
      </div>
      <p className="text-xs text-muted-foreground">{t("metrics.lastSecond")}</p>
      {series.length > 1 && (
        <section className="space-y-1">
          <h3 className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">{t("metrics.p95Trend")}</h3>
          <Sparkline xs={series.map((p) => p.t)} ys={series.map((p) => p.p95Ms)} label={t("metrics.p95Trend")} testId="p95-sparkline" />
        </section>
      )}
      {busiest.length > 0 && (
        <section className="space-y-2">
          <h3 className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">{t("metrics.busiest")}</h3>
          <ul className="space-y-2">
            {busiest.map((b) => {
              const node = byId.get(b.id);
              const name = node ? node.label.trim() || flavourName(t, node.flavour) : b.id;
              return (
                <li key={b.id} className="space-y-1" data-testid="busiest-row">
                  <div className="flex justify-between gap-2 text-sm">
                    <span className="truncate">{name}</span>
                    <span className="font-mono tabular-nums">{b.down ? t("metrics.down") : `${b.util}%`}</span>
                  </div>
                  <div className="h-1.5 rounded bg-muted" aria-hidden>
                    <div
                      className={b.util >= 85 ? "h-full rounded bg-terracotta" : b.util >= 60 ? "h-full rounded bg-amber" : "h-full rounded bg-green"}
                      style={{ width: `${b.util}%` }}
                    />
                  </div>
                </li>
              );
            })}
          </ul>
        </section>
      )}
      {info && info.sample_factor < 1 && (
        <p className="text-xs text-muted-foreground">{t("metrics.sampled", { pct: n0.format(info.sample_factor * 100) })}</p>
      )}
    </div>
  );
}
