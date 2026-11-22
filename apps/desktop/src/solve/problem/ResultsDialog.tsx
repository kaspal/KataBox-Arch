// Results of a submission: the verdict, the score parts, each test (hidden ones too, by name
// and numbers), the bottleneck of a failing test with "Replay at failure", and a placeholder
// for the AI review (M4). Every verdict colour comes with an icon and words.
import { useTranslation } from "react-i18next";
import { AlertTriangle, CheckCircle2, Lightbulb, Lock, RotateCcw, Sparkles, Timer, Wallet, XCircle } from "lucide-react";
import type { SubmitResult, TestResult, Verdict } from "@/bindings";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Markdown } from "@/components/Markdown";
import { cn } from "@/lib/utils";
import { CompareButton } from "@/solve/compare/CompareButton";
import { useUnlockHint } from "@/solve/problem/BriefPanel";
import { useProblemMode } from "@/solve/problem/context";
import { flavourName, kindName } from "@/solve/registry";
import { useEditor } from "@/solve/store";

const VERDICT: Record<Verdict, { icon: typeof CheckCircle2; tone: string }> = {
  accepted: { icon: CheckCircle2, tone: "text-green" },
  dropped_requests: { icon: XCircle, tone: "text-destructive" },
  slo_breach: { icon: Timer, tone: "text-amber-ink" },
  over_budget: { icon: Wallet, tone: "text-amber-ink" },
  invalid_design: { icon: AlertTriangle, tone: "text-destructive" },
};

export function VerdictChip({ v }: { v: Verdict }) {
  const { t } = useTranslation();
  const { icon: Icon, tone } = VERDICT[v];
  return (
    <span className={cn("inline-flex items-center gap-1 text-sm font-semibold", tone)} data-testid="verdict">
      <Icon aria-hidden className="size-4" />
      {t(`results.verdict.${v}`)}
    </span>
  );
}

function ScoreBar({ label, value, max }: { label: string; value: number; max: number }) {
  return (
    <div className="space-y-1">
      <div className="flex justify-between text-xs">
        <span>{label}</span>
        <span className="font-mono tabular-nums">
          {value} / {max}
        </span>
      </div>
      <div className="h-1.5 rounded bg-muted" aria-hidden>
        <div className="h-full rounded bg-primary" style={{ width: `${(value / max) * 100}%` }} />
      </div>
    </div>
  );
}

function TestRow({ test, onReplay }: { test: TestResult; onReplay: (t: TestResult) => void }) {
  const { t, i18n } = useTranslation();
  const es = i18n.language.startsWith("es");
  const nodes = useEditor((s) => s.doc.nodes);
  const n1 = new Intl.NumberFormat(i18n.language, { maximumFractionDigits: 1 });
  const pct = new Intl.NumberFormat(i18n.language, { style: "percent", maximumFractionDigits: 2 });
  const b = test.bottleneck;
  const card = b ? nodes.find((n) => n.id === b.node) : undefined;
  const bName = b ? (card ? card.label.trim() || flavourName(t, card.flavour) : kindName(t, b.kind)) : "";
  return (
    <li className="space-y-2 rounded-md border p-3" data-testid="test-result">
      <div className="flex items-center justify-between gap-2">
        <span className="flex items-center gap-2 font-medium">
          {es ? test.name_es : test.name}
          {test.hidden && (
            <span className="inline-flex items-center gap-1 text-xs font-normal text-muted-foreground">
              <Lock aria-hidden className="size-3" />
              {t("brief.hidden")}
            </span>
          )}
        </span>
        <VerdictChip v={test.verdict} />
      </div>
      {test.verdict !== "invalid_design" && (
        <dl className={`grid gap-2 text-xs ${test.limits.p99_ms != null ? "grid-cols-4" : "grid-cols-3"}`}>
          <div>
            <dt className="text-muted-foreground">{t("metrics.p95")}</dt>
            <dd className="font-mono tabular-nums">
              {n1.format(test.p95_ms)} ms{test.limits.p95_ms != null && ` / ${n1.format(test.limits.p95_ms)}`}
            </dd>
          </div>
          {test.limits.p99_ms != null && (
            <div data-testid="p99">
              <dt className="text-muted-foreground">{t("metrics.p99")}</dt>
              <dd className="font-mono tabular-nums">
                {n1.format(test.p99_ms)} ms / {n1.format(test.limits.p99_ms)}
              </dd>
            </div>
          )}
          <div>
            <dt className="text-muted-foreground">{t("metrics.errorRate")}</dt>
            <dd className="font-mono tabular-nums">
              {pct.format(test.error_rate)}
              {test.limits.error_rate != null && ` / ${pct.format(test.limits.error_rate)}`}
            </dd>
          </div>
          <div>
            <dt className="text-muted-foreground">{t("metrics.availability")}</dt>
            <dd className="font-mono tabular-nums">{pct.format(test.availability)}</dd>
          </div>
        </dl>
      )}
      {b && (
        <div className="flex items-center justify-between gap-2 rounded bg-muted/60 p-2 text-xs" data-testid="bottleneck">
          <span>{t(`results.bottleneck.${b.reason}`, { name: bName, at: b.from_s })}</span>
          {test.replay_at_s !== null && (
            <Button size="sm" variant="outline" className="h-7 gap-1" onClick={() => onReplay(test)}>
              <RotateCcw aria-hidden className="size-3.5" />
              {t("results.replay")}
            </Button>
          )}
        </div>
      )}
    </li>
  );
}

export function ResultsDialog({
  result,
  onClose,
  onReplay,
}: {
  result: SubmitResult | null;
  onClose: () => void;
  onReplay: (t: TestResult) => void;
}) {
  const { t, i18n } = useTranslation();
  const usd = new Intl.NumberFormat(i18n.language, { style: "currency", currency: "USD", maximumFractionDigits: 0 });
  const j = result?.judgement;
  return (
    <Dialog open={!!result} onOpenChange={(o) => !o && onClose()}>
      <DialogContent className="max-h-[85vh] overflow-y-auto sm:max-w-2xl" data-testid="results">
        {j && result && (
          <>
            <DialogHeader>
              <DialogTitle className="flex items-center justify-between gap-3">
                {t("results.title")}
                <VerdictChip v={j.verdict} />
              </DialogTitle>
              <DialogDescription>
                {result.first_solve
                  ? t("results.firstSolve")
                  : t("results.summary", {
                      passed: j.tests.filter((x) => x.verdict === "accepted").length,
                      total: j.tests.length,
                      cost: usd.format(j.cost_month),
                    })}
              </DialogDescription>
            </DialogHeader>
            {j.lint.length > 0 && (
              <ul className="list-disc space-y-1 pl-5 text-sm text-destructive" data-testid="lint">
                {j.lint.map((l) => (
                  <li key={l}>{l}</li>
                ))}
              </ul>
            )}
            <section className="space-y-2 rounded-md border p-3" data-testid="score">
              <div className="flex items-baseline justify-between">
                <h3 className="font-semibold">{t("results.score")}</h3>
                <span className="font-mono text-2xl tabular-nums">{j.score.total}</span>
              </div>
              <ScoreBar label={t("results.correctness")} value={j.score.correctness} max={70} />
              <ScoreBar label={t("results.cost", { cost: usd.format(j.cost_month), ref: usd.format(j.ref_cost) })} value={j.score.cost} max={20} />
              <ScoreBar label={t("results.latency")} value={j.score.latency} max={10} />
              <p className="text-xs text-muted-foreground">{t("results.best", { score: result.best_score })}</p>
            </section>
            <ul className="space-y-2">
              {j.tests.map((x) => (
                <TestRow key={x.id} test={x} onReplay={onReplay} />
              ))}
            </ul>
            <ResultsExtras accepted={j.verdict === "accepted"} />
            <section className="flex items-start gap-3 rounded-md border border-dashed p-3 text-sm text-muted-foreground" data-testid="ai-review">
              <Sparkles aria-hidden className="mt-0.5 size-4 shrink-0" />
              <p>{t("results.aiLater")}</p>
            </section>
            <DialogFooter>
              <Button onClick={onClose}>{t("common.close")}</Button>
            </DialogFooter>
          </>
        )}
      </DialogContent>
    </Dialog>
  );
}

/** On the Solve page: the next hint after a failed submission, and Compare once solved. */
function ResultsExtras({ accepted }: { accepted: boolean }) {
  const { t, i18n } = useTranslation();
  const problem = useProblemMode();
  const { unlock, busy } = useUnlockHint(problem?.id ?? "");
  if (!problem) return null;
  const { brief } = problem;
  const hints = i18n.language.startsWith("es") ? brief.hints_es : brief.hints_en;
  const last = hints.at(-1);
  const offerHint = !accepted && (brief.hints_unlocked < 3 || last);
  if (!offerHint && !(brief.solved && brief.reference)) return null;
  return (
    <div className="space-y-3">
      {offerHint && (
        <section className="space-y-2 rounded-md border p-3" data-testid="results-hint">
          <h3 className="flex items-center gap-2 text-sm font-semibold">
            <Lightbulb aria-hidden className="size-4" />
            {t("results.stuck")}
          </h3>
          {last && (
            <div className="text-sm">
              <div className="mb-1 text-xs font-semibold text-muted-foreground">{t("brief.hint", { n: brief.hints_unlocked })}</div>
              <Markdown>{last}</Markdown>
            </div>
          )}
          {brief.hints_unlocked < 3 && (
            <Button variant="outline" size="sm" className="gap-2" disabled={busy} onClick={() => void unlock(brief.hints_unlocked + 1)}>
              <Lightbulb aria-hidden className="size-4" />
              {t("brief.showHint", { n: brief.hints_unlocked + 1 })}
            </Button>
          )}
        </section>
      )}
      {brief.solved && brief.reference && <CompareButton reference={brief.reference} />}
    </div>
  );
}
