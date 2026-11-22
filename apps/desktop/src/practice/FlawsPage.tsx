// /flaws: Find the flaw (M5.3). Ten working-looking designs, each with one flaw and a test
// it fails. A drill opens on the Solve layout as the problem `flaw--<id>`: run it, find the
// flaw, fix it, and Submit until every test passes.
import { useTranslation } from "react-i18next";
import { Link } from "@tanstack/react-router";
import { Bug, CheckCircle2, Circle } from "lucide-react";
import { EmptyState, Page } from "@/components/Page";
import { Skeleton } from "@/components/ui/skeleton";
import { describeError } from "@/lib/errors";
import { useFlaws } from "@/lib/queries";
import { kindName } from "@/solve/registry";

export function FlawsPage() {
  const { t, i18n } = useTranslation();
  const flaws = useFlaws();
  const es = i18n.language.startsWith("es");
  if (flaws.error) {
    return (
      <Page title={t("flaws.title")}>
        <EmptyState icon={Bug} title={t("guide.unavailable")} body={describeError(i18n, flaws.error).summary} />
      </Page>
    );
  }
  if (!flaws.data) {
    return (
      <Page title={t("flaws.title")}>
        <Skeleton className="h-80 w-full" />
      </Page>
    );
  }
  const solved = flaws.data.filter((f) => f.solved).length;
  return (
    <Page title={t("flaws.title")}>
      <p className="mb-2 max-w-2xl text-sm text-muted-foreground">{t("flaws.intro")}</p>
      <p className="mb-6 text-sm" data-testid="flaws-progress">
        {t("flaws.progress", { solved, total: flaws.data.length })}
      </p>
      <ul className="divide-y rounded-xl border bg-card" data-testid="flaw-list">
        {flaws.data.map((f) => (
          <li key={f.id}>
            <Link
              to="/problems/$id"
              params={{ id: `flaw--${f.id}` }}
              className="grid grid-cols-[3rem_1fr_auto] items-center gap-4 px-4 py-3 hover:bg-accent/50"
              data-testid="flaw-row"
            >
              <span className="font-mono text-sm text-muted-foreground">#{f.number}</span>
              <span className="min-w-0">
                <span className="block truncate font-semibold">{es ? f.title_es : f.title}</span>
                <span className="block truncate text-xs text-muted-foreground">{f.kinds.map((k) => kindName(t, k)).join(" · ")}</span>
              </span>
              {f.solved ? (
                <span className="inline-flex items-center gap-1 text-xs font-semibold text-green">
                  <CheckCircle2 aria-hidden className="size-4" />
                  {t("flaws.fixed")}
                </span>
              ) : (
                <span className="inline-flex items-center gap-1 text-xs text-muted-foreground">
                  <Circle aria-hidden className="size-4" />
                  {t("flaws.open")}
                </span>
              )}
            </Link>
          </li>
        ))}
      </ul>
    </Page>
  );
}
