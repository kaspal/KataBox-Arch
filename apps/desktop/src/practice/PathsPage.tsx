// /paths: learning paths (M5.4). Five ordered tracks through the Guide, concept cards,
// patterns, problems and drills, with progress from what you've already done. Following a
// path makes Home's "Recommended next" its next unfinished item.
import { PrintMenu } from "@/print/PrintMenu";
import { useTranslation } from "react-i18next";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import { CheckCircle2, Circle, Flag, Route as RouteIcon } from "lucide-react";
import type { PathItem, PathView } from "@/bindings";
import { EmptyState, Page } from "@/components/Page";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { api } from "@/lib/api";
import { describeError } from "@/lib/errors";
import { usePaths } from "@/lib/queries";
import { kindName } from "@/solve/registry";

export const ACTIVE_PATH = "paths.active";

/** The followed path's id (a setting), and a setter. */
export function useActivePath() {
  const qc = useQueryClient();
  const q = useQuery({ queryKey: ["setting", ACTIVE_PATH], queryFn: () => api.getSetting(ACTIVE_PATH) });
  const id = typeof q.data === "string" ? q.data : null;
  const set = async (next: string | null) => {
    qc.setQueryData(["setting", ACTIVE_PATH], next);
    await api.setSetting(ACTIVE_PATH, next).catch(() => {});
  };
  return { id, set };
}

/** An item's name in the reader's language. */
export function useItemLabel() {
  const { t, i18n } = useTranslation();
  const es = i18n.language.startsWith("es");
  return (item: PathItem): string => {
    const content = (es ? item.title_es : item.title) ?? item.target;
    switch (item.kind) {
      case "guide":
        return t("paths.item.guide", { name: kindName(t, item.target as never) });
      case "concept":
        return t("paths.item.concept", { name: kindName(t, item.target as never) });
      case "pattern":
        return t("paths.item.pattern", { name: content });
      case "problem":
        return t("paths.item.problem", { name: content });
      case "estimate":
        return t("paths.item.estimate", { name: content });
      case "flaw":
        return t("paths.item.flaw", { name: content });
      case "replay":
        return t("paths.item.replay", { name: content });
      case "quiz": {
        const [what, id] = item.target.split(":");
        const title = es ? item.title_es : item.title;
        return t("paths.item.quiz", { name: what === "guide" ? kindName(t, id as never) : (title ?? id ?? item.target) });
      }
    }
  };
}

/** Where an item opens. */
export function ItemLink({ item, children, className, testId }: { item: PathItem; children: React.ReactNode; className?: string; testId?: string }) {
  const common = { className, "data-testid": testId };
  switch (item.kind) {
    case "guide":
    case "concept":
      return (
        <Link to="/guide/$kind" params={{ kind: item.target as never }} {...common}>
          {children}
        </Link>
      );
    case "pattern":
      return (
        <Link to="/patterns/$id" params={{ id: item.target }} {...common}>
          {children}
        </Link>
      );
    case "problem":
      return (
        <Link to="/problems/$id" params={{ id: item.target }} {...common}>
          {children}
        </Link>
      );
    case "estimate":
      return (
        <Link to="/estimate" search={{ problem: item.target }} {...common}>
          {children}
        </Link>
      );
    case "flaw":
      return (
        <Link to="/problems/$id" params={{ id: `flaw--${item.target}` }} {...common}>
          {children}
        </Link>
      );
    case "replay":
      return (
        <Link to="/replays/$id" params={{ id: item.target }} {...common}>
          {children}
        </Link>
      );
    case "quiz": {
      const [what, id] = item.target.split(":");
      return what === "guide" ? (
        <Link to="/guide/$kind" params={{ kind: id as never }} {...common}>
          {children}
        </Link>
      ) : (
        <Link to="/patterns/$id" params={{ id: id ?? "" }} {...common}>
          {children}
        </Link>
      );
    }
  }
}

function PathCard({ path, active, onFollow }: { path: PathView; active: boolean; onFollow: () => void }) {
  const { t, i18n } = useTranslation();
  const label = useItemLabel();
  const es = i18n.language.startsWith("es");
  const done = path.items.filter((i) => i.done).length;
  const total = path.items.length;
  const head = `path-${path.id}`;
  return (
    <section aria-labelledby={head} className="space-y-3 rounded-xl border bg-card p-5" data-testid="path">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div className="space-y-1">
          <h2 id={head} className="font-display text-2xl font-semibold">
            {es ? path.title_es : path.title}
          </h2>
          <p className="max-w-2xl text-sm text-muted-foreground">{es ? path.blurb_es : path.blurb}</p>
        </div>
        <div className="flex items-center gap-2">
        <PrintMenu what={`path:${path.id}`} name={es ? path.title_es : path.title} />
        {active ? (
          <span className="inline-flex items-center gap-1.5 text-sm font-semibold text-green" data-testid="path-following">
            <Flag aria-hidden className="size-4" />
            {t("paths.following")}
          </span>
        ) : (
          <Button variant="outline" size="sm" onClick={onFollow}>
            {t("paths.follow")}
          </Button>
        )}
        </div>
      </div>
      <div className="space-y-1">
        <div
          className="h-2 overflow-hidden rounded-full bg-muted"
          role="progressbar"
          aria-label={t("paths.progressLabel", { name: es ? path.title_es : path.title })}
          aria-valuemin={0}
          aria-valuemax={total}
          aria-valuenow={done}
        >
          <div className="h-full bg-green" style={{ width: `${total ? (100 * done) / total : 0}%` }} />
        </div>
        <p className="text-xs text-muted-foreground" data-testid="path-progress">
          {t("paths.progress", { done, total })}
        </p>
      </div>
      <ol className="grid gap-1 sm:grid-cols-2">
        {path.items.map((item, i) => (
          <li key={`${item.kind}:${item.target}`}>
            <ItemLink item={item} className="flex items-center gap-2 rounded-md px-2 py-1 text-sm hover:bg-accent/60" testId="path-item">
              {item.done ? (
                <CheckCircle2 aria-label={t("paths.done")} className="size-4 shrink-0 text-green" />
              ) : (
                <Circle aria-label={t("paths.todo")} className="size-4 shrink-0 text-muted-foreground" />
              )}
              <span className="font-mono text-xs text-muted-foreground">{i + 1}.</span>
              <span className="truncate">{label(item)}</span>
            </ItemLink>
          </li>
        ))}
      </ol>
    </section>
  );
}

export function PathsPage() {
  const { t, i18n } = useTranslation();
  const paths = usePaths();
  const active = useActivePath();
  if (paths.error) {
    return (
      <Page title={t("paths.title")}>
        <EmptyState icon={RouteIcon} title={t("guide.unavailable")} body={describeError(i18n, paths.error).summary} />
      </Page>
    );
  }
  if (!paths.data) {
    return (
      <Page title={t("paths.title")}>
        <Skeleton className="h-80 w-full" />
      </Page>
    );
  }
  return (
    <Page title={t("paths.title")}>
      <p className="mb-6 max-w-2xl text-sm text-muted-foreground">{t("paths.intro")}</p>
      <div className="space-y-6">
        {paths.data.map((p) => (
          <PathCard key={p.id} path={p} active={active.id === p.id} onFollow={() => void active.set(p.id)} />
        ))}
      </div>
    </Page>
  );
}
