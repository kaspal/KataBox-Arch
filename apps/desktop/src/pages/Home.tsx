// Home (matches the Home mockup's frame): the hero, New / Open, the first-run tour offer,
// practice (continue, recommended, streak), the samples gallery and the recent designs.
import { lazy, Suspense } from "react";
import { useTranslation } from "react-i18next";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { Link, useNavigate } from "@tanstack/react-router";
import type { ProblemSummary } from "@/bindings";
import { DifficultyBadge, useLocalized } from "@/pages/Problems";
import { FilePlus2, FileText, FolderOpen, PlayCircle, Trash2, X } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Page } from "@/components/Page";
import { Skeleton } from "@/components/ui/skeleton";
import { api } from "@/lib/api";
import { useProblems, useProgress, useRecentFiles, useSamples } from "@/lib/queries";
import { useFileActions } from "@/solve/files";

// The gallery draws designs with React Flow: load it apart from Home's own code.
const HomeSamples = lazy(() => import("@/tour/HomeSamples").then((m) => ({ default: m.HomeSamples })));

const TOUR_OFFERED = "tour.offered";

/** Offered once: take the tour, or not now. Either answer is remembered; the tour stays in Samples. */
function TourOffer() {
  const { t, i18n } = useTranslation();
  const qc = useQueryClient();
  const navigate = useNavigate();
  const offered = useQuery({ queryKey: ["setting", TOUR_OFFERED], queryFn: () => api.getSetting(TOUR_OFFERED) });
  const tour = useSamples().data?.find((s) => s.tour);
  if (!tour || offered.isLoading || offered.data === true) return null;
  const done = async () => {
    qc.setQueryData(["setting", TOUR_OFFERED], true);
    await api.setSetting(TOUR_OFFERED, true).catch(() => {});
  };
  const title = i18n.language.startsWith("es") ? tour.title_es : tour.title;
  return (
    <section className="flex flex-wrap items-center justify-between gap-4 rounded-xl border bg-card p-4" aria-labelledby="tour-offer-title" data-testid="tour-offer">
      <div className="space-y-1">
        <h2 id="tour-offer-title" className="font-semibold">
          {t("home.tourTitle")}
        </h2>
        <p className="text-sm text-muted-foreground">{t("home.tourBody", { title })}</p>
      </div>
      <div className="flex gap-2">
        <Button
          className="gap-2"
          onClick={() => {
            void done();
            void navigate({ to: "/samples/$id", params: { id: tour.id } });
          }}
        >
          <PlayCircle aria-hidden className="size-4" />
          {t("home.tourStart")}
        </Button>
        <Button variant="ghost" className="gap-2" onClick={() => void done()}>
          <X aria-hidden className="size-4" />
          {t("home.tourLater")}
        </Button>
      </div>
    </section>
  );
}

function when(ms: number, lang: string): string {
  return new Intl.DateTimeFormat(lang, { dateStyle: "medium", timeStyle: "short" }).format(new Date(ms));
}

/** Continue the last attempted problem, and the next one to try. Hidden without content. */
function PracticeCards() {
  const { t } = useTranslation();
  const problems = useProblems();
  const progress = useProgress();
  const loc = useLocalized();
  const find = (id: string | null | undefined) => problems.data?.find((p) => p.id === id);
  const cont = find(progress.data?.continue_with);
  const next = find(progress.data?.recommended);
  if (!cont && !next) return null;
  const card = (p: ProblemSummary, label: string, testId: string) => (
    <Link
      to="/problems/$id"
      params={{ id: p.id }}
      className="flex flex-col gap-1 rounded-xl border bg-card p-4 shadow-[0_1px_0_var(--desk)] hover:bg-accent/50"
      data-testid={testId}
    >
      <span className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">{label}</span>
      <span className="font-semibold">
        #{p.number} {loc.title(p)}
      </span>
      <span className="text-xs text-muted-foreground">{loc.teaches(p)}</span>
      <DifficultyBadge d={p.difficulty} />
    </Link>
  );
  return (
    <section aria-labelledby="practice-title" className="space-y-4">
      <h2 id="practice-title" className="font-display text-3xl font-semibold">
        {t("home.practice")}
      </h2>
      <div className="grid grid-cols-[repeat(auto-fill,minmax(260px,1fr))] gap-4">
        {cont && card(cont, t("home.continue"), "home-continue")}
        {next && next.id !== cont?.id && card(next, t("home.recommended"), "home-recommended")}
      </div>
      {progress.data && (
        <p className="text-sm text-muted-foreground" data-testid="home-streak">
          {t("home.streak", { count: progress.data.streak })}
        </p>
      )}
    </section>
  );
}

export function Home() {
  const { t, i18n } = useTranslation();
  const files = useFileActions();
  const recents = useRecentFiles();
  return (
    <Page
      eyebrow={t("home.eyebrow")}
      title={t("home.title")}
      actions={
        <>
          <Button variant="outline" className="gap-2 bg-card" onClick={() => void files.open()}>
            <FolderOpen aria-hidden className="size-4" />
            {t("files.openEllipsis")}
          </Button>
          <Button className="gap-2" onClick={() => void files.newDesign()}>
            <FilePlus2 aria-hidden className="size-4" />
            {t("home.newDesign")}
          </Button>
        </>
      }
    >
      <TourOffer />
      <PracticeCards />
      <Suspense fallback={<Skeleton className="h-56 w-full" />}>
        <HomeSamples />
      </Suspense>
      <section aria-labelledby="recent-title" className="space-y-4">
        <h2 id="recent-title" className="font-display text-3xl font-semibold">
          {t("home.recent")}
        </h2>
        {recents.isLoading && <Skeleton className="h-28 w-full" />}
        {recents.data?.length === 0 && (
          <div className="rounded-xl border border-dashed bg-card px-6 py-10 text-center text-muted-foreground">
            {t("home.noRecent")}
          </div>
        )}
        <ul className="grid grid-cols-[repeat(auto-fill,minmax(260px,1fr))] gap-4">
          {(recents.data ?? []).map((r) => (
            <li
              key={r.path}
              className="flex flex-col gap-3 rounded-xl border bg-card p-4 shadow-[0_1px_0_var(--desk)] data-[missing=true]:opacity-70"
              data-missing={!r.exists}
              data-testid="recent-file"
            >
              <div className="flex items-start gap-3">
                <span className="flex size-9 shrink-0 items-center justify-center rounded-lg bg-muted text-muted-foreground" aria-hidden>
                  <FileText className="size-4.5" />
                </span>
                <div className="min-w-0">
                  <h3 className="truncate font-semibold">{r.name}</h3>
                  <p className="truncate font-mono text-xs text-muted-foreground" title={r.path}>
                    {r.path}
                  </p>
                  <p className="text-xs text-muted-foreground">
                    {r.exists ? when(r.opened_at, i18n.language) : t("home.missing")}
                  </p>
                </div>
              </div>
              <div className="mt-auto flex gap-2">
                {r.exists && (
                  <Button size="sm" onClick={() => void files.openRecent(r.path)} aria-label={t("home.openNamed", { name: r.name })}>
                    {t("files.openShort")}
                  </Button>
                )}
                <Button
                  size="sm"
                  variant="ghost"
                  className="gap-1.5 text-muted-foreground"
                  onClick={() => void files.forgetRecent(r.path)}
                  aria-label={t("home.forgetNamed", { name: r.name })}
                >
                  <Trash2 aria-hidden className="size-3.5" />
                  {t("home.forget")}
                </Button>
              </div>
            </li>
          ))}
        </ul>
      </section>
    </Page>
  );
}
