// The problem catalogue: search, difficulty and topic filters, status and best score, plus
// "Continue" (the last attempted, unsolved problem) and "Random problem" (an unsolved one).
import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { Link, useNavigate } from "@tanstack/react-router";
import { CheckCircle2, CircleDashed, CircleDot, List, Package, Play, Ruler, Shuffle } from "lucide-react";
import type { Difficulty, ProblemSummary } from "@/bindings";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Skeleton } from "@/components/ui/skeleton";
import { EmptyState, Page } from "@/components/Page";
import { describeError } from "@/lib/errors";
import { useEstimateStatuses, useProblems, useProgress } from "@/lib/queries";
import { cn } from "@/lib/utils";
import { ImportPackButton } from "@/packs/PackImport";

const DIFFICULTIES: Difficulty[] = ["easy", "medium", "hard"];

export function useLocalized() {
  const { i18n } = useTranslation();
  const es = i18n.language.startsWith("es");
  return {
    es,
    title: (p: { title_en: string; title_es: string }) => (es ? p.title_es : p.title_en),
    teaches: (p: { teaches_en: string; teaches_es: string }) => (es ? p.teaches_es : p.teaches_en),
  };
}

/** Difficulty as text with a colour that only reinforces it. */
export function DifficultyBadge({ d }: { d: Difficulty }) {
  const { t } = useTranslation();
  const tone = d === "easy" ? "text-green" : d === "medium" ? "text-amber-ink" : "text-terracotta-ink";
  return <span className={cn("text-xs font-semibold", tone)}>{t(`problems.difficulty.${d}`)}</span>;
}

export function StatusMark({ status }: { status: string | null }) {
  const { t } = useTranslation();
  if (status === "solved") {
    return (
      <span className="inline-flex items-center gap-1 text-xs text-green">
        <CheckCircle2 aria-hidden className="size-3.5" />
        {t("problems.status.solved")}
      </span>
    );
  }
  if (status === "attempted") {
    return (
      <span className="inline-flex items-center gap-1 text-xs text-amber-ink">
        <CircleDot aria-hidden className="size-3.5" />
        {t("problems.status.attempted")}
      </span>
    );
  }
  return (
    <span className="inline-flex items-center gap-1 text-xs text-muted-foreground">
      <CircleDashed aria-hidden className="size-3.5" />
      {t("problems.status.new")}
    </span>
  );
}

function normalise(s: string) {
  return s
    .toLowerCase()
    .normalize("NFD")
    .replace(/[̀-ͯ]/g, "");
}

export function ProblemsPage() {
  const { t, i18n } = useTranslation();
  const navigate = useNavigate();
  const problems = useProblems();
  // Estimation drills done per problem (scored lightly: a mark here, never in the judge's score).
  const statuses = useEstimateStatuses();
  const estimated = new Map<string, number>();
  for (const s of statuses.data ?? []) if (s.done) estimated.set(s.problem_id, (estimated.get(s.problem_id) ?? 0) + 1);
  const progress = useProgress();
  const loc = useLocalized();
  const [query, setQuery] = useState("");
  const [difficulty, setDifficulty] = useState<Difficulty | null>(null);
  const [tag, setTag] = useState<string | null>(null);
  const [source, setSource] = useState<"all" | "builtin" | "packs">("all");

  const all = useMemo(() => problems.data ?? [], [problems.data]);
  const tags = useMemo(() => [...new Set(all.flatMap((p) => p.tags))].sort(), [all]);
  const q = normalise(query.trim());
  const shown = all.filter(
    (p) =>
      (!difficulty || p.difficulty === difficulty) &&
      (!tag || p.tags.includes(tag)) &&
      (source === "all" || (source === "packs") === (p.pack !== null)) &&
      (!q || normalise(`${p.title_en} ${p.title_es} ${p.teaches_en} ${p.teaches_es} ${p.tags.join(" ")}`).includes(q)),
  );
  const hasPacks = all.some((p) => p.pack !== null);
  const unsolved = all.filter((p) => p.status !== "solved");
  const continueWith = all.find((p) => p.id === progress.data?.continue_with);
  const open = (p: ProblemSummary) => void navigate({ to: "/problems/$id", params: { id: p.id } });

  if (problems.error) {
    return (
      <Page title={t("nav.problems")}>
        <EmptyState icon={List} title={t("problems.unavailable")} body={describeError(i18n, problems.error).summary} />
      </Page>
    );
  }
  return (
    <Page
      title={t("nav.problems")}
      actions={
        <>
          {continueWith && (
            <Button className="gap-2" onClick={() => open(continueWith)}>
              <Play aria-hidden className="size-4" />
              {t("problems.continue", { title: loc.title(continueWith) })}
            </Button>
          )}
          <Button
            variant="outline"
            className="gap-2 bg-card"
            disabled={unsolved.length === 0}
            onClick={() => open(unsolved[Math.floor(Math.random() * unsolved.length)])}
          >
            <Shuffle aria-hidden className="size-4" />
            {t("problems.random")}
          </Button>
          <ImportPackButton />
        </>
      }
    >
      <div className="space-y-3">
        <Input
          type="search"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          placeholder={t("problems.search")}
          aria-label={t("problems.search")}
          className="max-w-md bg-card"
        />
        <div className="flex flex-wrap gap-2" role="group" aria-label={t("problems.filterDifficulty")}>
          {[null, ...DIFFICULTIES].map((d) => (
            <Button
              key={d ?? "all"}
              size="sm"
              variant={difficulty === d ? "default" : "outline"}
              aria-pressed={difficulty === d}
              onClick={() => setDifficulty(d)}
            >
              {d ? t(`problems.difficulty.${d}`) : t("problems.all")}
            </Button>
          ))}
        </div>
        {hasPacks && (
          <div className="flex flex-wrap gap-2" role="group" aria-label={t("problems.source.label")}>
            {(["all", "builtin", "packs"] as const).map((s) => (
              <Button key={s} size="sm" variant={source === s ? "default" : "outline"} aria-pressed={source === s} onClick={() => setSource(s)}>
                {t(`problems.source.${s}`)}
              </Button>
            ))}
          </div>
        )}
        {tags.length > 0 && (
          <div className="flex flex-wrap gap-2" role="group" aria-label={t("problems.filterTopic")}>
            {tags.map((g) => (
              <Button
                key={g}
                size="sm"
                variant={tag === g ? "default" : "ghost"}
                aria-pressed={tag === g}
                onClick={() => setTag(tag === g ? null : g)}
                className="h-7 rounded-full px-3 text-xs"
              >
                #{g}
              </Button>
            ))}
          </div>
        )}
      </div>
      {problems.isLoading && <Skeleton className="h-40 w-full" />}
      <ul className="divide-y rounded-xl border bg-card" data-testid="problem-list">
        {shown.map((p) => (
          <li key={p.id}>
            <Link
              to="/problems/$id"
              params={{ id: p.id }}
              className="grid grid-cols-[3rem_1fr_auto] items-center gap-4 px-4 py-3 hover:bg-accent/50"
              data-testid="problem-row"
            >
              <span className="font-mono text-sm text-muted-foreground">#{p.number}</span>
              <span className="min-w-0">
                <span className="block truncate font-semibold">{loc.title(p)}</span>
                <span className="block truncate text-xs text-muted-foreground">{loc.teaches(p)}</span>
                {p.pack !== null && (
                  <span className="mt-0.5 inline-flex items-center gap-1 text-xs text-muted-foreground" data-testid="pack-badge">
                    <Package aria-hidden className="size-3.5" />
                    {t("problems.fromPack", { name: p.pack })}
                  </span>
                )}
              </span>
              <span className="flex items-center gap-4">
                <DifficultyBadge d={p.difficulty} />
                {estimated.has(p.id) && (
                  <span className="inline-flex items-center gap-1 text-xs text-muted-foreground" title={t("problems.estimated")} data-testid="estimated-mark">
                    <Ruler aria-hidden className="size-3.5" />
                    <span className="sr-only">{t("problems.estimated")}</span>
                    {estimated.get(p.id)}
                  </span>
                )}
                <StatusMark status={p.status} />
                <span className="w-16 text-right font-mono text-xs tabular-nums text-muted-foreground">
                  {p.best_score === null ? "—" : t("problems.best", { score: p.best_score })}
                </span>
              </span>
            </Link>
          </li>
        ))}
        {!problems.isLoading && shown.length === 0 && (
          <li className="px-4 py-8 text-center text-sm text-muted-foreground">{t("problems.none")}</li>
        )}
      </ul>
    </Page>
  );
}
