// The Brief tab: the statement, the numbers to meet, the traffic, external providers, the
// tests (hidden ones by name only), hints revealed tier by tier, and the editorial once solved.
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { useQueryClient } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import { Calculator, ChevronDown, Lightbulb, Lock, Ruler } from "lucide-react";
import type { ProblemBrief } from "@/bindings";
import { useReportError } from "@/app/dialogs";
import { Markdown } from "@/components/Markdown";
import { Button } from "@/components/ui/button";
import { api } from "@/lib/api";
import { queryKeys, useDrills, useEstimateStatuses, useSubmissions } from "@/lib/queries";
import { EstimateDrill } from "@/practice/EstimateDrill";
import { DifficultyBadge, StatusMark, useLocalized } from "@/pages/Problems";
import { CompareButton } from "@/solve/compare/CompareButton";
import { CostSpeedChart } from "@/solve/problem/CostSpeedChart";

function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <section className="space-y-2">
      <h3 className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">{title}</h3>
      {children}
    </section>
  );
}

function Stat({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-md border bg-card px-3 py-2">
      <div className="text-xs text-muted-foreground">{label}</div>
      <div className="font-mono text-sm tabular-nums">{value}</div>
    </div>
  );
}

/** Reveal hint `tier` of problem `id` (recorded in Rust), then refresh the brief. */
export function useUnlockHint(id: string) {
  const report = useReportError();
  const qc = useQueryClient();
  const [busy, setBusy] = useState(false);
  const unlock = async (tier: number) => {
    setBusy(true);
    try {
      await api.unlockHint(id, tier);
      await qc.invalidateQueries({ queryKey: queryKeys.brief(id) });
    } catch (e) {
      report(e);
    } finally {
      setBusy(false);
    }
  };
  return { unlock, busy };
}

export function BriefPanel({ brief }: { brief: ProblemBrief }) {
  const { t, i18n } = useTranslation();
  const loc = useLocalized();
  const p = brief.summary;
  const { unlock, busy } = useUnlockHint(p.id);
  const n0 = new Intl.NumberFormat(i18n.language, { maximumFractionDigits: 0 });
  const pct = new Intl.NumberFormat(i18n.language, { style: "percent", maximumFractionDigits: 2 });
  const usd = new Intl.NumberFormat(i18n.language, { style: "currency", currency: "USD", maximumFractionDigits: 0 });
  const hints = loc.es ? brief.hints_es : brief.hints_en;
  // A Find-the-flaw drill (M5.3) runs as the problem `flaw--<id>`: no difficulty, no topic.
  const drill = p.id.startsWith("flaw--");

  return (
    <div className="space-y-5" data-testid="brief">
      <div className="space-y-1">
        <div className="flex items-center gap-3">
          <span className="font-mono text-xs text-muted-foreground">#{p.number}</span>
          {drill ? <span className="text-xs font-semibold text-terracotta-ink">{t("flaws.drill")}</span> : <DifficultyBadge d={p.difficulty} />}
          <StatusMark status={p.status} />
        </div>
        {drill ? <p className="text-xs text-muted-foreground">{t("flaws.task")}</p> : <p className="text-xs text-muted-foreground">{t("brief.teaches", { what: loc.teaches(p) })}</p>}
      </div>
      <Markdown>{loc.es ? brief.statement_es : brief.statement_en}</Markdown>
      <Section title={t("brief.targets")}>
        <div className="grid grid-cols-2 gap-2">
          <Stat label={t("brief.peak")} value={`${n0.format(brief.nfr.peak_rps)} ${t("units.rps")}`} />
          <Stat label={t("brief.p95")} value={`≤ ${n0.format(brief.nfr.p95_ms)} ms`} />
          <Stat label={t("brief.availability")} value={`≥ ${pct.format(brief.nfr.availability)}`} />
          <Stat label={t("brief.budget")} value={`${usd.format(brief.nfr.budget_usd)}${t("metrics.perMonth")}`} />
        </div>
        <Link to="/numbers" className="inline-flex items-center gap-1.5 text-xs font-semibold text-primary underline-offset-4 hover:underline" data-testid="numbers-link">
          <Calculator aria-hidden className="size-3.5" />
          {t("brief.numbers")}
        </Link>
      </Section>
      <EstimateFirst problemId={p.id} />
      <Section title={t("brief.traffic")}>
        <ul className="space-y-1 text-sm">
          {brief.request_types.map((r) => (
            <li key={r.id} className="flex justify-between gap-2">
              <span>{(loc.es ? r.name_es : r.name) ?? r.id}</span>
              <span className="font-mono tabular-nums text-muted-foreground">{pct.format(r.share)}</span>
            </li>
          ))}
        </ul>
      </Section>
      {brief.externals.length > 0 && (
        <Section title={t("brief.externals")}>
          <ul className="space-y-1 text-sm" data-testid="externals">
            {brief.externals.map((x) => (
              <li key={x.name} className="flex justify-between gap-2">
                <span className="font-mono">{x.name}</span>
                <span className="text-xs text-muted-foreground">
                  {t("brief.externalLatency", { median: n0.format(x.median_ms), p99: n0.format(x.p99_ms) })}
                  {x.rate_limit !== null && ` · ${t("brief.externalLimit", { limit: n0.format(x.rate_limit) })}`}
                </span>
              </li>
            ))}
          </ul>
        </Section>
      )}
      <Section title={t("brief.tests")}>
        <ul className="space-y-1 text-sm" data-testid="tests">
          {brief.tests.map((x) => (
            <li key={x.id} className="flex items-center justify-between gap-2">
              <span>{loc.es ? x.name_es : x.name}</span>
              {x.hidden ? (
                <span className="inline-flex items-center gap-1 text-xs text-muted-foreground">
                  <Lock aria-hidden className="size-3" />
                  {t("brief.hidden")}
                </span>
              ) : (
                <span className="text-xs text-muted-foreground">{t("brief.seconds", { s: x.duration_s })}</span>
              )}
            </li>
          ))}
        </ul>
      </Section>
      <Section title={t("brief.hints")}>
        {hints.map((h, i) => (
          <div key={i} className="rounded-md border bg-card p-3" data-testid="hint">
            <div className="mb-1 text-xs font-semibold text-muted-foreground">{t("brief.hint", { n: i + 1 })}</div>
            <Markdown>{h}</Markdown>
          </div>
        ))}
        {brief.hints_unlocked < 3 && (
          <Button variant="outline" size="sm" className="gap-2" disabled={busy} onClick={() => void unlock(brief.hints_unlocked + 1)}>
            <Lightbulb aria-hidden className="size-4" />
            {t("brief.showHint", { n: brief.hints_unlocked + 1 })}
          </Button>
        )}
      </Section>
      {brief.solved && <SolvedSection brief={brief} />}
      {brief.solved && (loc.es ? brief.editorial_es : brief.editorial_en) && (
        <Section title={t("brief.editorial")}>
          <Markdown>{(loc.es ? brief.editorial_es : brief.editorial_en) ?? ""}</Markdown>
        </Section>
      )}
    </div>
  );
}

/**
 * "Estimate first" (M5.2): the problem's back-of-the-envelope drills, scored lightly: a count
 * of drills done, never part of the judge's score. Folded until opened.
 */
function EstimateFirst({ problemId }: { problemId: string }) {
  const { t } = useTranslation();
  const drills = useDrills();
  const statuses = useEstimateStatuses();
  const [open, setOpen] = useState(false);
  const mine = (drills.data ?? []).filter((d) => d.problem_id === problemId);
  if (mine.length === 0) return null;
  const done = new Set((statuses.data ?? []).filter((s) => s.problem_id === problemId && s.done).map((s) => s.drill_id));
  const count = mine.filter((d) => done.has(d.id)).length;
  return (
    <section className="space-y-2" data-testid="estimate-first">
      <button
        type="button"
        className="flex w-full items-center justify-between gap-2 text-left"
        aria-expanded={open}
        onClick={() => setOpen((o) => !o)}
      >
        <span className="flex items-center gap-1.5 text-xs font-semibold uppercase tracking-wide text-muted-foreground">
          <Ruler aria-hidden className="size-3.5" />
          {t("brief.estimateFirst")}
        </span>
        <span className="flex items-center gap-1 text-xs text-muted-foreground" data-testid="estimate-count">
          {t("brief.estimated", { done: count, total: mine.length })}
          <ChevronDown aria-hidden className={`size-3.5 transition-transform ${open ? "rotate-180" : ""}`} />
        </span>
      </button>
      {open && (
        <div className="space-y-3">
          {mine.map((d) => (
            <EstimateDrill key={d.id} drill={d} done={done.has(d.id)} />
          ))}
        </div>
      )}
    </section>
  );
}

/** Once solved: compare with the reference, and where your accepted designs sit on cost and speed. */
function SolvedSection({ brief }: { brief: ProblemBrief }) {
  const { t } = useTranslation();
  const subs = useSubmissions(brief.summary.id);
  return (
    <Section title={t("brief.compare")}>
      {brief.reference && <CompareButton reference={brief.reference} />}
      <CostSpeedChart submissions={subs.data ?? []} reference={brief.reference_point} />
    </Section>
  );
}
