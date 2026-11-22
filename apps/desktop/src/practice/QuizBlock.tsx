// Quick-check quizzes (M5.5): three questions at the end of each Guide entry and pattern.
// Answers are judged in Rust, which only then reveals the right option and why; a missed
// question comes back for review (tomorrow, then at growing intervals).
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { useQueryClient } from "@tanstack/react-query";
import { CheckCircle2, CircleX } from "lucide-react";
import type { QuizItem, QuizResult } from "@/bindings";
import { useReportError } from "@/app/dialogs";
import { api } from "@/lib/api";
import { localDay } from "@/lib/dates";
import { queryKeys, useQuiz } from "@/lib/queries";
import { cn } from "@/lib/utils";

/** "Back on 26 Nov" in the reader's language. */
function dueLabel(due: string, lang: string): string {
  const [y, m, d] = due.split("-").map(Number);
  return new Intl.DateTimeFormat(lang, { day: "numeric", month: "short" }).format(new Date(y!, m! - 1, d!));
}

/** One question: pick an option, see whether it was right and why. */
export function QuizQuestionCard({ item, number, onAnswered }: { item: QuizItem; number?: number; onAnswered?: (r: QuizResult) => void }) {
  const { t, i18n } = useTranslation();
  const es = i18n.language.startsWith("es");
  const report = useReportError();
  const qc = useQueryClient();
  const [picked, setPicked] = useState<number | null>(null);
  const [result, setResult] = useState<QuizResult | null>(null);
  const options = es ? item.options_es : item.options;
  const pick = async (i: number) => {
    if (result) return;
    setPicked(i);
    try {
      const r = await api.answerQuiz(item.id, i, localDay());
      setResult(r);
      onAnswered?.(r);
      await Promise.all([qc.invalidateQueries({ queryKey: queryKeys.reviewsDue }), qc.invalidateQueries({ queryKey: queryKeys.paths })]);
    } catch (e) {
      setPicked(null);
      report(e);
    }
  };
  const head = `${item.id}-q`;
  return (
    <fieldset className="space-y-2 rounded-lg border bg-card p-4" aria-labelledby={head} data-testid="quiz-question">
      <legend id={head} className="sr-only">
        {es ? item.q_es : item.q}
      </legend>
      <p className="text-sm font-semibold" aria-hidden>
        {number !== undefined && <span className="mr-1 font-mono text-muted-foreground">{number}.</span>}
        {es ? item.q_es : item.q}
      </p>
      <div className="grid gap-1.5">
        {options.map((o, i) => {
          const right = result !== null && i === result.answer;
          const wrong = result !== null && i === picked && !result.correct;
          return (
            <button
              key={o}
              type="button"
              disabled={result !== null}
              aria-pressed={picked === i}
              onClick={() => void pick(i)}
              className={cn(
                "flex items-center gap-2 rounded-md border px-3 py-2 text-left text-sm hover:bg-accent/60 disabled:cursor-default",
                right && "border-green bg-green/10",
                wrong && "border-terracotta bg-terracotta/10",
              )}
            >
              {right && <CheckCircle2 aria-label={t("quiz.right")} className="size-4 shrink-0 text-green" />}
              {wrong && <CircleX aria-label={t("quiz.wrong")} className="size-4 shrink-0 text-terracotta-ink" />}
              <span>{o}</span>
            </button>
          );
        })}
      </div>
      {result && (
        <div className="space-y-1 text-sm" role="status" aria-live="polite" data-testid="quiz-feedback">
          <p className={result.correct ? "font-semibold text-green" : "font-semibold text-terracotta-ink"}>{result.correct ? t("quiz.correct") : t("quiz.incorrect")}</p>
          <p className="text-muted-foreground">{es ? result.why_es : result.why}</p>
          {result.due_on && <p className="text-xs text-muted-foreground">{t("quiz.backOn", { date: dueLabel(result.due_on, i18n.language) })}</p>}
        </div>
      )}
    </fieldset>
  );
}

/** The three questions at the end of a Guide entry (`guide:<kind>`) or a pattern (`pattern:<id>`). */
export function QuizBlock({ page }: { page: string }) {
  const { t } = useTranslation();
  const quiz = useQuiz(page);
  if (!quiz.data || quiz.data.length === 0) return null;
  return (
    <section className="space-y-3" aria-labelledby={`${page}-quiz`} data-testid="quiz">
      <div>
        <h2 id={`${page}-quiz`} className="font-display text-xl font-semibold">
          {t("quiz.title")}
        </h2>
        <p className="text-sm text-muted-foreground">{t("quiz.intro")}</p>
      </div>
      {quiz.data.map((item, i) => (
        <QuizQuestionCard key={item.id} item={item} number={i + 1} />
      ))}
    </section>
  );
}
