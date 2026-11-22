// Home (matches the Home mockup's frame): the hero, New / Open, and the recent designs.
// The samples gallery, progress and recommendations arrive with M2–M3.
import { useTranslation } from "react-i18next";
import { FilePlus2, FileText, FolderOpen, Trash2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Page } from "@/components/Page";
import { Skeleton } from "@/components/ui/skeleton";
import { useRecentFiles } from "@/lib/queries";
import { useFileActions } from "@/solve/files";

function when(ms: number, lang: string): string {
  return new Intl.DateTimeFormat(lang, { dateStyle: "medium", timeStyle: "short" }).format(new Date(ms));
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
