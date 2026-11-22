// /review: today's spaced review (M5.5). Up to ten due questions, one at a time, each with
// its explanation after answering. The list is fixed when the session starts, so answering
// doesn't reshuffle it.
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Link } from "@tanstack/react-router";
import { CalendarCheck, ChevronRight } from "lucide-react";
import type { QuizItem } from "@/bindings";
import { EmptyState, Page } from "@/components/Page";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { describeError } from "@/lib/errors";
import { useReviewsDue } from "@/lib/queries";
import { QuizQuestionCard } from "@/practice/QuizBlock";
import { kindName } from "@/solve/registry";

/** Where a question comes from, for the review card: "Cache" or a pattern's id. */
export function useQuestionSource() {
  const { t } = useTranslation();
  return (id: string) => {
    const [kind, target] = id.split(":");
    return kind === "guide" ? kindName(t, target as never) : t("quiz.fromPattern", { id: target });
  };
}

export function ReviewPage() {
  const { t, i18n } = useTranslation();
  const due = useReviewsDue();
  const source = useQuestionSource();
  const [session, setSession] = useState<QuizItem[] | null>(null);
  const [at, setAt] = useState(0);
  const [answered, setAnswered] = useState(false);
  if (due.error) {
    return (
      <Page title={t("quiz.reviewTitle")}>
        <EmptyState icon={CalendarCheck} title={t("guide.unavailable")} body={describeError(i18n, due.error).summary} />
      </Page>
    );
  }
  if (!due.data && !session) {
    return (
      <Page title={t("quiz.reviewTitle")}>
        <Skeleton className="h-60 w-full" />
      </Page>
    );
  }
  const items = session ?? due.data ?? [];
  if (!session && items.length > 0) setSession(items);
  const current = items[at];
  return (
    <Page title={t("quiz.reviewTitle")}>
      {items.length === 0 ? (
        <EmptyState icon={CalendarCheck} title={t("quiz.nothingDue")} body={t("quiz.nothingDueBody")} />
      ) : current ? (
        <div className="max-w-2xl space-y-4">
          <p className="text-sm text-muted-foreground" data-testid="review-position">
            {t("quiz.position", { n: at + 1, total: items.length })} · {source(current.id)}
          </p>
          <QuizQuestionCard key={current.id} item={current} onAnswered={() => setAnswered(true)} />
          {answered && (
            <Button
              className="gap-2"
              onClick={() => {
                setAnswered(false);
                setAt((i) => i + 1);
              }}
            >
              {at + 1 < items.length ? t("quiz.next") : t("quiz.finish")}
              <ChevronRight aria-hidden className="size-4" />
            </Button>
          )}
        </div>
      ) : (
        <div className="space-y-4">
          <EmptyState icon={CalendarCheck} title={t("quiz.sessionDone")} body={t("quiz.sessionDoneBody", { count: items.length })} />
          <Link to="/" className="block text-center font-semibold text-primary underline-offset-4 hover:underline">
            {t("nav.home")}
          </Link>
        </div>
      )}
    </Page>
  );
}
