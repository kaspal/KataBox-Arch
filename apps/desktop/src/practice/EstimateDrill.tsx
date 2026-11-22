// One estimation drill (M5.2): the question with its inputs, an answer box, and after a check
// the verdict, the computed answer and the worked solution. Answers are judged in Rust, which
// only then reveals the answer; a drill can be tried again.
import { useId, useState } from "react";
import { useTranslation } from "react-i18next";
import { useQueryClient } from "@tanstack/react-query";
import { CheckCircle2, CircleAlert, RotateCcw } from "lucide-react";
import type { Drill, EstimateCheck } from "@/bindings";
import { useReportError } from "@/app/dialogs";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { api } from "@/lib/api";
import { queryKeys } from "@/lib/queries";

/** `{name}` → the input's value and `{answer}` → the answer, in the reader's number format. */
export function fillText(text: string, inputs: Partial<Record<string, number>>, lang: string, answer?: number): string {
  const fmt = (v: number) =>
    new Intl.NumberFormat(lang, { maximumFractionDigits: Math.abs(v) >= 100 ? 0 : Math.abs(v) >= 1 ? 1 : 3 }).format(v);
  return text.replace(/\{([a-z0-9_]+)\}/gi, (whole, name: string) => {
    const v = name === "answer" ? answer : inputs[name];
    return v === undefined ? whole : fmt(v);
  });
}

/** The unit as shown after a number. */
export function unitLabel(t: (k: "units.rps") => string, unit: string): string {
  if (unit === "rps") return t("units.rps");
  if (unit === "count") return "";
  return unit === "us" ? "µs" : unit;
}

export function EstimateDrill({ drill, done }: { drill: Drill; done?: boolean }) {
  const { t, i18n } = useTranslation();
  const es = i18n.language.startsWith("es");
  const report = useReportError();
  const qc = useQueryClient();
  const id = useId();
  const [value, setValue] = useState("");
  const [result, setResult] = useState<EstimateCheck | null>(null);
  const [busy, setBusy] = useState(false);
  const unit = unitLabel(t, drill.unit);
  const parsed = Number(value.replace(/[\s,_]/g, ""));
  const valid = value.trim() !== "" && Number.isFinite(parsed) && parsed > 0;

  const check = async () => {
    if (!valid) return;
    setBusy(true);
    try {
      setResult(await api.checkEstimate(drill.problem_id, drill.id, parsed));
      await qc.invalidateQueries({ queryKey: queryKeys.estimateStatuses });
    } catch (e) {
      report(e);
    } finally {
      setBusy(false);
    }
  };

  const verdict = result && (
    <div className="space-y-2" role="status" aria-live="polite" data-testid="estimate-result">
      <p className={result.ok ? "flex items-center gap-1.5 font-semibold text-green" : "flex items-center gap-1.5 font-semibold text-amber-ink"}>
        {result.ok ? <CheckCircle2 aria-hidden className="size-4" /> : <CircleAlert aria-hidden className="size-4" />}
        {t(`estimate.closeness.${result.closeness}`)}
      </p>
      <p className="text-sm">
        {t("estimate.answerWas")}{" "}
        <span className="font-mono tabular-nums">
          {fillText("{answer}", {}, i18n.language, result.answer)}
          {unit && ` ${unit}`}
        </span>
      </p>
      <p className="rounded-md bg-muted/60 p-3 text-sm leading-relaxed">{fillText(es ? result.solution_es : result.solution, result.inputs, i18n.language, result.answer)}</p>
    </div>
  );

  return (
    <div className="space-y-3 rounded-lg border bg-card p-4" data-testid="estimate-drill">
      <div className="flex items-start justify-between gap-3">
        <p className="text-sm leading-relaxed">{fillText(es ? drill.prompt_es : drill.prompt, drill.inputs, i18n.language)}</p>
        {done && (
          <span className="inline-flex shrink-0 items-center gap-1 text-xs font-semibold text-green" data-testid="estimate-done">
            <CheckCircle2 aria-hidden className="size-3.5" />
            {t("estimate.done")}
          </span>
        )}
      </div>
      <form
        className="flex flex-wrap items-center gap-2"
        onSubmit={(e) => {
          e.preventDefault();
          void check();
        }}
      >
        <label htmlFor={id} className="sr-only">
          {t("estimate.yourAnswer")}
        </label>
        <Input id={id} inputMode="decimal" value={value} onChange={(e) => setValue(e.target.value)} className="w-40 bg-background font-mono" placeholder={t("estimate.placeholder")} />
        {unit && <span className="text-sm text-muted-foreground">{unit}</span>}
        <Button type="submit" size="sm" disabled={!valid || busy}>
          {t("estimate.check")}
        </Button>
        {result && (
          <Button
            type="button"
            size="sm"
            variant="ghost"
            className="gap-1"
            onClick={() => {
              setResult(null);
              setValue("");
            }}
          >
            <RotateCcw aria-hidden className="size-3.5" />
            {t("estimate.again")}
          </Button>
        )}
      </form>
      {verdict}
    </div>
  );
}
