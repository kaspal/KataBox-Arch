// /replays: outage replays (M10). Each is a well-known kind of incident, told plainly, then
// replayed on the design as it was ("Watch it happen", in the walkthrough player) and fixed
// on the Solve layout as the problem `outage--<id>` ("Fix it"), judged on the incident, a
// normal day and an aftershock.
import { useTranslation } from "react-i18next";
import { Link, useParams } from "@tanstack/react-router";
import { CheckCircle2, ChevronLeft, Circle, History, Wrench } from "lucide-react";
import { EmptyState, Page } from "@/components/Page";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { describeError } from "@/lib/errors";
import { DifficultyBadge } from "@/pages/Problems";
import { useOutages, useOutageStory, useRegistry } from "@/lib/queries";
import { kindName } from "@/solve/registry";
import { Player } from "@/tour/Player";

export function ReplaysPage() {
  const { t, i18n } = useTranslation();
  const outages = useOutages();
  const es = i18n.language.startsWith("es");
  if (outages.error) {
    return (
      <Page title={t("replays.title")}>
        <EmptyState icon={History} title={t("guide.unavailable")} body={describeError(i18n, outages.error).summary} />
      </Page>
    );
  }
  if (!outages.data) {
    return (
      <Page title={t("replays.title")}>
        <Skeleton className="h-80 w-full" />
      </Page>
    );
  }
  const fixed = outages.data.filter((o) => o.solved).length;
  return (
    <Page title={t("replays.title")}>
      <p className="mb-2 max-w-2xl text-sm text-muted-foreground">{t("replays.intro")}</p>
      <p className="mb-6 text-sm" data-testid="replays-progress">
        {t("replays.progress", { fixed, total: outages.data.length })}
      </p>
      {outages.data.length === 0 ? (
        <EmptyState icon={History} title={t("replays.none")} body={t("replays.noneBody")} />
      ) : (
        <ul className="divide-y rounded-xl border bg-card" data-testid="replay-list">
          {outages.data.map((o) => (
            <li key={o.id}>
              <Link
                to="/replays/$id"
                params={{ id: o.id }}
                className="grid grid-cols-[3rem_1fr_auto_auto] items-center gap-4 px-4 py-3 hover:bg-accent/50"
                data-testid="replay-row"
              >
                <span className="font-mono text-sm text-muted-foreground">#{o.number}</span>
                <span className="min-w-0">
                  <span className="block truncate font-semibold">{es ? o.title_es : o.title}</span>
                  <span className="block truncate text-xs text-muted-foreground">{es ? o.summary_es : o.summary}</span>
                  <span className="block truncate text-xs text-muted-foreground">{o.kinds.map((k) => kindName(t, k)).join(" · ")}</span>
                </span>
                <DifficultyBadge d={o.difficulty} />
                {o.solved ? (
                  <span className="inline-flex items-center gap-1 text-xs font-semibold text-green">
                    <CheckCircle2 aria-hidden className="size-4" />
                    {t("replays.fixed")}
                  </span>
                ) : (
                  <span className="inline-flex items-center gap-1 text-xs text-muted-foreground">
                    <Circle aria-hidden className="size-4" />
                    {t("replays.open")}
                  </span>
                )}
              </Link>
            </li>
          ))}
        </ul>
      )}
    </Page>
  );
}

/** /replays/$id: what happened, then the walkthrough, with "Fix it" always at hand. */
export function ReplayPage() {
  const { t, i18n } = useTranslation();
  const { id } = useParams({ from: "/replays/$id" });
  const story = useOutageStory(id);
  const registry = useRegistry();
  const es = i18n.language.startsWith("es");
  if (story.error) {
    return (
      <Page title={t("replays.title")}>
        <EmptyState icon={History} title={t("guide.unavailable")} body={describeError(i18n, story.error).summary} />
      </Page>
    );
  }
  if (!story.data || !registry.data) {
    return <Skeleton className="m-4 h-[calc(100%-2rem)]" />;
  }
  const s = story.data;
  const title = es ? s.summary.title_es : s.summary.title;
  const w = s.walkthrough;
  return (
    <div className="flex h-full min-h-0 flex-col">
      <section className="border-b px-6 py-3" aria-label={t("replays.whatHappened")} data-testid="replay-story">
        <h2 className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">{t("replays.whatHappened")}</h2>
        <p className="mt-1 max-w-4xl text-sm">{es ? s.story_es : s.story}</p>
        {s.summary.solved && (
          <p className="mt-2 max-w-4xl text-sm" data-testid="replay-lesson">
            <span className="font-semibold">{t("replays.lesson")} </span>
            {es ? s.lesson_es : s.lesson}
          </p>
        )}
      </section>
      <Player
        story={{ title, problemId: w.def.problem ?? null, steps: w.def.steps, snapshots: w.snapshots }}
        registry={registry.data}
        header={
          <>
            <Link to="/replays" className="flex items-center gap-1 text-sm text-muted-foreground hover:text-foreground">
              <ChevronLeft aria-hidden className="size-4" />
              {t("replays.title")}
            </Link>
            <span aria-hidden className="text-muted-foreground">
              /
            </span>
            <h1 className="truncate font-display text-2xl font-semibold">{title}</h1>
            <Button asChild size="sm" className="ml-auto gap-2" data-testid="replay-fix">
              <Link to="/problems/$id" params={{ id: `outage--${id}` }}>
                <Wrench aria-hidden className="size-4" />
                {t("replays.fixIt")}
              </Link>
            </Button>
          </>
        }
      />
    </div>
  );
}
