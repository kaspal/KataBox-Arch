// /guide: the Component Guide index (guide-index mockup). Entries grouped by category, a
// search over names and one-liners, the "Start here" six, and a read mark on each entry.
import { PrintMenu } from "@/print/PrintMenu";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Link } from "@tanstack/react-router";
import { BookOpen, Check, Search } from "lucide-react";
import type { ComponentKind, GuideSummary, Registry } from "@/bindings";
import { EmptyState, Page } from "@/components/Page";
import { Input } from "@/components/ui/input";
import { Skeleton } from "@/components/ui/skeleton";
import { describeError } from "@/lib/errors";
import { useDecideFlows, useGuide, useRegistry } from "@/lib/queries";
import { GlyphTile } from "@/solve/icons";
import { categoryName, kindName, kindSpec } from "@/solve/registry";

/** The six pieces almost every design uses (plan M3.4). */
export const START_HERE: ComponentKind[] = ["lb", "app", "cache", "sql", "queue", "cdn"];

function Tile({ g, registry }: { g: GuideSummary; registry: Registry }) {
  const { t, i18n } = useTranslation();
  const spec = kindSpec(registry, g.kind);
  const name = kindName(t, g.kind);
  return (
    <li>
      <Link
        to="/guide/$kind"
        params={{ kind: g.kind }}
        className="flex items-center gap-3 rounded-xl border bg-card p-3 transition-colors hover:border-primary focus-visible:outline-2 focus-visible:outline-ring"
        data-testid="guide-tile"
      >
        <GlyphTile icon={spec.icon} category={spec.category} className="size-10 [&_svg]:size-5" />
        <span className="min-w-0 flex-1">
          <span className="block font-semibold leading-tight">{name}</span>
          <span className="block text-xs text-muted-foreground">{i18n.language.startsWith("es") ? g.one_liner_es : g.one_liner}</span>
        </span>
        {g.read && (
          <span className="shrink-0 text-green" title={t("guide.read")}>
            <Check aria-hidden className="size-4" />
            <span className="sr-only">{t("guide.read")}</span>
          </span>
        )}
      </Link>
    </li>
  );
}

export function GuideIndex() {
  const { t, i18n } = useTranslation();
  const guide = useGuide();
  const registry = useRegistry();
  const [q, setQ] = useState("");
  const flows = useDecideFlows();
  const es = i18n.language.startsWith("es");

  if (guide.error) {
    return (
      <Page title={t("guide.title")}>
        <EmptyState icon={BookOpen} title={t("guide.unavailable")} body={describeError(i18n, guide.error).summary} />
      </Page>
    );
  }
  if (!guide.data || !registry.data) {
    return (
      <Page title={t("guide.title")}>
        <Skeleton className="h-96 w-full" />
      </Page>
    );
  }
  const reg = registry.data;
  const all = guide.data;
  const needle = q.trim().toLowerCase();
  const shown = needle
    ? all.filter((g) => `${kindName(t, g.kind)} ${es ? g.one_liner_es : g.one_liner} ${g.kind}`.toLowerCase().includes(needle))
    : all;
  const read = all.filter((g) => g.read).length;
  return (
    <Page
      title={t("guide.title")}
      actions={
        <div className="flex items-center gap-4">
        <Link to="/patterns" className="text-sm font-semibold whitespace-nowrap text-primary underline underline-offset-4">
          {t("patterns.title")}
        </Link>
        <Link to="/glossary" className="text-sm font-semibold whitespace-nowrap text-primary underline underline-offset-4">
          {t("glossary.title")}
        </Link>
        <PrintMenu what="guide" name={t("guide.title")} label={t("print.wholeGuide")} />
        <label className="relative block w-72">
          <Search aria-hidden className="pointer-events-none absolute top-1/2 left-3 size-4 -translate-y-1/2 text-muted-foreground" />
          <Input type="search" value={q} onChange={(e) => setQ(e.target.value)} placeholder={t("guide.search")} aria-label={t("guide.searchLabel")} className="bg-card pl-9" />
        </label>
        </div>
      }
    >
      <p className="-mt-4 text-muted-foreground" data-testid="guide-progress">
        {t("guide.subtitle", { read, total: all.length })}
      </p>
      {!needle && (
        <section className="flex flex-wrap items-center gap-x-6 gap-y-3 rounded-xl border bg-card p-4" aria-labelledby="start-here" data-testid="start-here">
          <div>
            <h2 id="start-here" className="font-display text-xl">
              {t("guide.startHere")}
            </h2>
            <p className="text-sm font-semibold">{t("guide.startHereBody")}</p>
          </div>
          <ul className="flex flex-wrap gap-2">
            {START_HERE.map((k) => {
              const spec = kindSpec(reg, k);
              const g = all.find((x) => x.kind === k);
              return (
                <li key={k}>
                  <Link to="/guide/$kind" params={{ kind: k }} className="flex items-center gap-2 rounded-full border bg-background py-1 pr-3 pl-1 text-sm font-medium hover:border-primary">
                    <GlyphTile icon={spec.icon} category={spec.category} className="size-7 rounded-full [&_svg]:size-4" />
                    {kindName(t, k)}
                    {g?.read && <Check aria-label={t("guide.read")} className="size-3.5 text-green" />}
                  </Link>
                </li>
              );
            })}
          </ul>
        </section>
      )}
      {!needle && (flows.data ?? []).length > 0 && (
        <section aria-labelledby="decide-h" className="space-y-2" data-testid="decide-list">
          <h2 id="decide-h" className="font-display text-xl">
            {t("decide.title")}
          </h2>
          <ul className="flex flex-wrap gap-2">
            {(flows.data ?? []).map((f) => (
              <li key={f.id}>
                <Link to="/decide/$id" params={{ id: f.id }} className="block rounded-full border bg-card px-4 py-1.5 text-sm font-medium hover:border-primary">
                  {es ? f.title_es : f.title}
                </Link>
              </li>
            ))}
          </ul>
        </section>
      )}
      {shown.length === 0 ? (
        <p className="text-muted-foreground">{t("guide.noMatch")}</p>
      ) : (
        <div className="grid gap-6 md:grid-cols-2 xl:grid-cols-5">
          {reg.categories.map((c) => {
            const items = shown.filter((g) => kindSpec(reg, g.kind).category === c);
            if (items.length === 0) return null;
            return (
              <section key={c} aria-labelledby={`cat-${c}`} className="space-y-2">
                <h2 id={`cat-${c}`} className="flex items-center gap-2 text-xs font-semibold tracking-wide uppercase">
                  <span aria-hidden className="size-2 rounded-sm" style={{ background: `var(--cat-${c})` }} />
                  {categoryName(t, c)}
                </h2>
                <ul className="space-y-2">
                  {items.map((g) => (
                    <Tile key={g.kind} g={g} registry={reg} />
                  ))}
                </ul>
              </section>
            );
          })}
        </div>
      )}
    </Page>
  );
}
