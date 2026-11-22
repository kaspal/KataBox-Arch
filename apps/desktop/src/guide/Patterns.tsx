// /patterns and /patterns/$id: the pattern library. Each pattern has a Guide-style page and a
// template; "Insert into the Sandbox" adds the template to the design being edited, centred
// in the view with fresh ids, as one undo step.
import { QuizBlock } from "@/practice/QuizBlock";
import { useTranslation } from "react-i18next";
import { useQueryClient } from "@tanstack/react-query";
import { Link, useNavigate, useParams } from "@tanstack/react-router";
import { Check, Circle, Layers, Plus, X } from "lucide-react";
import type { PatternPage as PatternPageData, Registry } from "@/bindings";
import { useReportError } from "@/app/dialogs";
import { Markdown } from "@/components/Markdown";
import { EmptyState, Page } from "@/components/Page";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { api } from "@/lib/api";
import { describeError } from "@/lib/errors";
import { usePattern, usePatterns, useProblems, useRegistry } from "@/lib/queries";
import { DifficultyBadge, useLocalized } from "@/pages/Problems";
import { requestAdd } from "@/solve/bridge";
import { GlyphTile } from "@/solve/icons";
import { kindName, kindSpec } from "@/solve/registry";
import { forkDesign } from "@/tour/fork";
import { DesignThumb } from "@/tour/SamplesPage";

export function PatternsIndex() {
  const { t, i18n } = useTranslation();
  const patterns = usePatterns();
  const registry = useRegistry();
  const es = i18n.language.startsWith("es");
  if (patterns.error) {
    return (
      <Page title={t("patterns.title")}>
        <EmptyState icon={Layers} title={t("patterns.unavailable")} body={describeError(i18n, patterns.error).summary} />
      </Page>
    );
  }
  if (!patterns.data || !registry.data) {
    return (
      <Page title={t("patterns.title")}>
        <Skeleton className="h-80 w-full" />
      </Page>
    );
  }
  const read = patterns.data.filter((p) => p.read).length;
  return (
    <Page title={t("patterns.title")} actions={<Link to="/guide" className="text-sm font-semibold text-primary underline underline-offset-4">{t("guide.title")}</Link>}>
      <p className="-mt-4 text-muted-foreground" data-testid="patterns-progress">
        {t("patterns.subtitle", { read, total: patterns.data.length })}
      </p>
      <ul className="grid gap-4 sm:grid-cols-2 xl:grid-cols-3">
        {patterns.data.map((p) => (
          <li key={p.id}>
            <Link
              to="/patterns/$id"
              params={{ id: p.id }}
              className="flex h-full flex-col gap-3 rounded-xl border bg-card p-3 transition-colors hover:border-primary focus-visible:outline-2 focus-visible:outline-ring"
              data-testid="pattern-card"
            >
              <DesignThumb design={p.template} registry={registry.data} label={es ? p.title_es : p.title} />
              <div className="space-y-1 px-1">
                <div className="flex items-baseline justify-between gap-2">
                  <h2 className="font-semibold">{es ? p.title_es : p.title}</h2>
                  {p.read && (
                    <span className="flex items-center gap-1 text-xs text-green">
                      <Check aria-hidden className="size-3.5" />
                      {t("guide.read")}
                    </span>
                  )}
                </div>
                <p className="text-sm text-muted-foreground">{es ? p.one_liner_es : p.one_liner}</p>
              </div>
            </Link>
          </li>
        ))}
      </ul>
    </Page>
  );
}

function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <section className="space-y-3 rounded-xl border bg-card p-5">
      <h2 className="font-display text-2xl font-semibold">{title}</h2>
      {children}
    </section>
  );
}

function Pattern({ page, registry }: { page: PatternPageData; registry: Registry }) {
  const { t, i18n } = useTranslation();
  const loc = useLocalized();
  const report = useReportError();
  const qc = useQueryClient();
  const navigate = useNavigate();
  const problems = useProblems();
  const e = page.entry;
  const es = i18n.language.startsWith("es");
  const L = <T,>(en: T, esv: T) => (es ? esv : en);
  const title = L(e.title, e.title_es);

  const insert = () => {
    requestAdd({ pattern: forkDesign(page.template, title) });
    void navigate({ to: "/sandbox" });
  };
  const toggleRead = async () => {
    try {
      await api.setPatternRead(e.id, !page.read);
      await qc.invalidateQueries({ queryKey: ["patterns"] });
    } catch (err) {
      report(err);
    }
  };

  return (
    <div className="mx-auto max-w-6xl space-y-6 px-8 py-8" data-testid="pattern">
      <nav aria-label={t("guide.breadcrumb")} className="text-sm">
        <Link to="/patterns" className="underline underline-offset-4">
          {t("patterns.title")}
        </Link>
        <span aria-hidden className="mx-2 text-muted-foreground">
          /
        </span>
        <span>{title}</span>
      </nav>
      <header className="flex flex-wrap items-start justify-between gap-4">
        <div>
          <h1 className="font-display text-5xl font-semibold">{title}</h1>
          <p className="text-lg text-muted-foreground">{L(e.one_liner, e.one_liner_es)}</p>
        </div>
        <div className="flex gap-2">
          <Button variant={page.read ? "outline" : "secondary"} className="gap-2" onClick={() => void toggleRead()} aria-pressed={page.read}>
            {page.read ? <Check aria-hidden className="size-4" /> : <Circle aria-hidden className="size-4" />}
            {page.read ? t("guide.markedRead") : t("guide.markRead")}
          </Button>
          <Button className="gap-2" onClick={insert}>
            <Plus aria-hidden className="size-4" />
            {t("patterns.insert")}
          </Button>
        </div>
      </header>
      <div className="grid gap-6 lg:grid-cols-[minmax(0,1fr)_22rem]">
        <div className="space-y-6">
          <Section title={t("patterns.problem")}>
            <p>{L(e.problem, e.problem_es)}</p>
          </Section>
          <Section title={t("patterns.howItWorks")}>
            <Markdown>{L(page.md_en, page.md_es)}</Markdown>
          </Section>
          <Section title={t("patterns.template")}>
            <DesignThumb design={page.template} registry={registry} label={t("patterns.template")} />
            <p className="text-sm text-muted-foreground">{t("patterns.templateHint")}</p>
          </Section>
          <div className="grid gap-6 sm:grid-cols-2">
            <section className="space-y-2 rounded-xl border bg-card p-5">
              <h2 className="font-semibold text-green">{t("guide.gains")}</h2>
              <ul className="space-y-2 text-sm">
                {L(e.gains, e.gains_es).map((s) => (
                  <li key={s} className="flex gap-2">
                    <Check aria-hidden className="mt-0.5 size-4 shrink-0 text-green" />
                    {s}
                  </li>
                ))}
              </ul>
            </section>
            <section className="space-y-2 rounded-xl border bg-card p-5">
              <h2 className="font-semibold text-destructive">{t("guide.costs")}</h2>
              <ul className="space-y-2 text-sm">
                {L(e.costs, e.costs_es).map((s) => (
                  <li key={s} className="flex gap-2">
                    <X aria-hidden className="mt-0.5 size-4 shrink-0 text-destructive" />
                    {s}
                  </li>
                ))}
              </ul>
            </section>
          </div>
        </div>
        <aside className="space-y-6">
          <section className="space-y-2 rounded-xl border bg-card p-5">
            <h2 className="font-semibold text-green">{t("guide.useWhen")}</h2>
            <ul className="space-y-2 text-sm">
              {L(e.use_when, e.use_when_es).map((s) => (
                <li key={s} className="flex gap-2">
                  <Check aria-hidden className="mt-0.5 size-4 shrink-0 text-green" />
                  {s}
                </li>
              ))}
            </ul>
          </section>
          <section className="space-y-2 rounded-xl border bg-card p-5" data-testid="pattern-sim">
            <h2 className="text-xs font-semibold tracking-wide uppercase">{t("guide.inSimulator")}</h2>
            <p className="text-sm">{L(e.in_simulator, e.in_simulator_es)}</p>
          </section>
          <section className="space-y-2 rounded-xl border bg-card p-5">
            <h2 className="text-xs font-semibold tracking-wide uppercase">{t("patterns.builtFrom")}</h2>
            <ul className="flex flex-wrap gap-2">
              {e.kinds.map((k) => {
                const s = kindSpec(registry, k);
                return (
                  <li key={k}>
                    <Link to="/guide/$kind" params={{ kind: k }} className="flex items-center gap-2 rounded-full border bg-background py-1 pr-3 pl-1 text-sm font-medium hover:border-primary">
                      <GlyphTile icon={s.icon} category={s.category} className="size-6 rounded-full [&_svg]:size-3.5" />
                      {kindName(t, k)}
                    </Link>
                  </li>
                );
              })}
            </ul>
          </section>
          {(e.practice ?? []).length > 0 && (
            <section className="space-y-2 rounded-xl border bg-card p-5">
              <h2 className="text-xs font-semibold tracking-wide uppercase">{t("guide.practice")}</h2>
              <ul className="divide-y">
                {(e.practice ?? []).map((pid) => {
                  const s = problems.data?.find((x) => x.id === pid);
                  return (
                    <li key={pid}>
                      <Link to="/problems/$id" params={{ id: pid }} className="flex items-center justify-between gap-2 py-2 text-sm font-semibold hover:underline">
                        {s ? loc.title(s) : pid}
                        {s && <DifficultyBadge d={s.difficulty} />}
                      </Link>
                    </li>
                  );
                })}
              </ul>
            </section>
          )}
        </aside>
      </div>
      <QuizBlock page={`pattern:${e.id}`} />
    </div>
  );
}

export function PatternPage() {
  const { t, i18n } = useTranslation();
  const { id } = useParams({ from: "/patterns/$id" });
  const page = usePattern(id);
  const registry = useRegistry();
  if (page.error) {
    return (
      <Page title={t("patterns.title")}>
        <EmptyState icon={Layers} title={t("patterns.unavailable")} body={describeError(i18n, page.error).summary} />
      </Page>
    );
  }
  if (!page.data || !registry.data) return <Skeleton className="m-8 h-96" />;
  return <Pattern page={page.data} registry={registry.data} />;
}
