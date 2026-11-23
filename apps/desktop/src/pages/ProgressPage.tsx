// Progress (M8.6): solved problems per track and difficulty, cost challenges, and the streak.
import { useTranslation } from "react-i18next";
import { BarChart3, Flame, Wallet } from "lucide-react";
import { EmptyState, Page } from "@/components/Page";
import { Skeleton } from "@/components/ui/skeleton";
import { useProblems, useProgress } from "@/lib/queries";
import { ProgressRing, TRACKS } from "@/pages/Problems";

export function Progress() {
  const { t } = useTranslation();
  const problems = useProblems();
  const progress = useProgress();
  if (problems.isLoading || progress.isLoading) {
    return (
      <Page title={t("nav.progress")}>
        <Skeleton className="h-40 w-full" />
      </Page>
    );
  }
  const all = problems.data ?? [];
  const p = progress.data;
  if (all.length === 0 || !p) {
    return (
      <Page title={t("nav.progress")}>
        <EmptyState icon={BarChart3} title={t("progress.empty")} body={t("progress.emptyBody")} />
      </Page>
    );
  }
  return (
    <Page title={t("nav.progress")}>
      <div className="grid gap-4 sm:grid-cols-3" data-testid="progress-tracks">
        {TRACKS.map((tr) => {
          const inTrack = all.filter((x) => (x.track || "core") === tr && x.pack === null);
          if (inTrack.length === 0) return null;
          return (
            <section key={tr} className="space-y-2 rounded-xl border bg-card p-4" aria-labelledby={`progress-${tr}`}>
              <h2 id={`progress-${tr}`} className="font-semibold">
                {t(`problems.track.${tr}`)}
              </h2>
              <ProgressRing solved={inTrack.filter((x) => x.status === "solved").length} total={inTrack.length} label={t("problems.solvedOf")} />
              <ul className="space-y-0.5 text-xs text-muted-foreground">
                {(["easy", "medium", "hard"] as const).map((d) => {
                  const ds = inTrack.filter((x) => x.difficulty === d);
                  return (
                    <li key={d}>
                      {t(`problems.difficulty.${d}`)}: {ds.filter((x) => x.status === "solved").length}/{ds.length}
                    </li>
                  );
                })}
              </ul>
            </section>
          );
        })}
      </div>
      <div className="flex flex-wrap gap-6 text-sm">
        <span className="inline-flex items-center gap-2" data-testid="progress-challenges">
          <Wallet aria-hidden className="size-4" />
          {t("progress.challenges", { solved: p.challenges_solved, total: p.challenges_total })}
        </span>
        <span className="inline-flex items-center gap-2" data-testid="progress-streak">
          <Flame aria-hidden className="size-4" />
          {t("progress.streak", { count: p.streak })}
        </span>
      </div>
    </Page>
  );
}
