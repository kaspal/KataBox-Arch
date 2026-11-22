// /decide/$id: a "Which one do I need?" guide, one question at a time, ending at a Guide
// entry. /glossary: the terms the app uses, searchable.
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Link, useParams } from "@tanstack/react-router";
import { BookA, ChevronRight, RotateCcw, Search, Signpost } from "lucide-react";
import type { DecideFlow, DecideOption, Registry } from "@/bindings";
import { EmptyState, Page } from "@/components/Page";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Skeleton } from "@/components/ui/skeleton";
import { describeError } from "@/lib/errors";
import { useDecideFlows, useGlossary, useRegistry } from "@/lib/queries";
import { GlyphTile } from "@/solve/icons";
import { kindName, kindSpec } from "@/solve/registry";

function Flow({ flow, registry }: { flow: DecideFlow; registry: Registry }) {
  const { t, i18n } = useTranslation();
  const es = i18n.language.startsWith("es");
  const L = (en: string, esv: string) => (es ? esv : en);
  // The answers given so far (question id, option), in order.
  const [path, setPath] = useState<{ q: string; o: DecideOption }[]>([]);
  const last = path.at(-1)?.o;
  const current = last?.next ?? (last ? null : flow.start);
  const question = flow.questions.find((q) => q.id === current);
  const result = last?.result ?? null;
  return (
    <div className="mx-auto max-w-3xl space-y-6 px-8 py-8" data-testid="decide">
      <nav aria-label={t("guide.breadcrumb")} className="text-sm">
        <Link to="/guide" className="underline underline-offset-4">
          {t("guide.title")}
        </Link>
        <span aria-hidden className="mx-2 text-muted-foreground">
          /
        </span>
        <span>{L(flow.title, flow.title_es)}</span>
      </nav>
      <header className="space-y-1">
        <h1 className="font-display text-4xl font-semibold">{L(flow.title, flow.title_es)}</h1>
        <p className="text-muted-foreground">{L(flow.intro, flow.intro_es)}</p>
      </header>
      {path.length > 0 && (
        <ol className="space-y-1 text-sm" aria-label={t("decide.answers")}>
          {path.map(({ q, o }) => {
            const qq = flow.questions.find((x) => x.id === q);
            return (
              <li key={q} className="flex flex-wrap gap-x-2 text-muted-foreground">
                <span>{qq ? L(qq.text, qq.text_es) : q}</span>
                <span className="font-semibold text-foreground">{L(o.label, o.label_es)}</span>
              </li>
            );
          })}
        </ol>
      )}
      {question && (
        <section className="space-y-3 rounded-xl border bg-card p-5" aria-labelledby="decide-q" data-testid="decide-question">
          <h2 id="decide-q" className="text-xl font-semibold">
            {L(question.text, question.text_es)}
          </h2>
          <div className="flex flex-col gap-2">
            {question.options.map((o) => (
              <Button key={o.label} variant="outline" className="h-auto justify-between py-3 text-left whitespace-normal" onClick={() => setPath((p) => [...p, { q: question.id, o }])}>
                {L(o.label, o.label_es)}
                <ChevronRight aria-hidden className="size-4 shrink-0" />
              </Button>
            ))}
          </div>
        </section>
      )}
      {result && last && (
        <section className="space-y-3 rounded-xl border bg-card p-5" aria-labelledby="decide-r" data-testid="decide-result">
          <h2 id="decide-r" className="text-xs font-semibold tracking-wide uppercase">
            {t("decide.result")}
          </h2>
          <Link to="/guide/$kind" params={{ kind: result }} className="flex items-center gap-3 rounded-lg border bg-background p-3 hover:border-primary">
            <GlyphTile icon={kindSpec(registry, result).icon} category={kindSpec(registry, result).category} className="size-10 [&_svg]:size-5" />
            <span className="font-display text-2xl font-semibold">{kindName(t, result)}</span>
            <ChevronRight aria-hidden className="ml-auto size-5" />
          </Link>
          <p>{L(last.why, last.why_es)}</p>
        </section>
      )}
      {path.length > 0 && (
        <Button variant="ghost" className="gap-2" onClick={() => setPath([])}>
          <RotateCcw aria-hidden className="size-4" />
          {t("decide.restart")}
        </Button>
      )}
    </div>
  );
}

export function DecidePage() {
  const { t, i18n } = useTranslation();
  const { id } = useParams({ from: "/decide/$id" });
  const flows = useDecideFlows();
  const registry = useRegistry();
  if (flows.error) {
    return (
      <Page title={t("decide.title")}>
        <EmptyState icon={Signpost} title={t("guide.unavailable")} body={describeError(i18n, flows.error).summary} />
      </Page>
    );
  }
  if (!flows.data || !registry.data) return <Skeleton className="m-8 h-80" />;
  const flow = flows.data.find((f) => f.id === id);
  if (!flow) {
    return (
      <Page title={t("decide.title")}>
        <EmptyState icon={Signpost} title={t("decide.notFound")} body={t("decide.notFoundBody")} />
      </Page>
    );
  }
  return <Flow key={flow.id} flow={flow} registry={registry.data} />;
}

export function GlossaryPage() {
  const { t, i18n } = useTranslation();
  const glossary = useGlossary();
  const [q, setQ] = useState("");
  const es = i18n.language.startsWith("es");
  if (glossary.error) {
    return (
      <Page title={t("glossary.title")}>
        <EmptyState icon={BookA} title={t("guide.unavailable")} body={describeError(i18n, glossary.error).summary} />
      </Page>
    );
  }
  if (!glossary.data) {
    return (
      <Page title={t("glossary.title")}>
        <Skeleton className="h-80 w-full" />
      </Page>
    );
  }
  const needle = q.trim().toLowerCase();
  const terms = [...glossary.data]
    .map((g) => ({ g, term: es ? g.term_es : g.term, def: es ? g.definition_es : g.definition }))
    .filter((x) => !needle || `${x.term} ${x.def} ${x.g.term}`.toLowerCase().includes(needle))
    .sort((a, b) => a.term.localeCompare(b.term, i18n.language));
  return (
    <Page
      title={t("glossary.title")}
      actions={
        <label className="relative block w-72">
          <Search aria-hidden className="pointer-events-none absolute top-1/2 left-3 size-4 -translate-y-1/2 text-muted-foreground" />
          <Input type="search" value={q} onChange={(e) => setQ(e.target.value)} placeholder={t("glossary.search")} aria-label={t("glossary.search")} className="bg-card pl-9" />
        </label>
      }
    >
      {terms.length === 0 ? (
        <p className="text-muted-foreground">{t("guide.noMatch")}</p>
      ) : (
        <dl className="divide-y rounded-xl border bg-card" data-testid="glossary">
          {terms.map(({ g, term, def }) => (
            <div key={g.id} id={g.id} className="grid gap-1 px-5 py-3 sm:grid-cols-[14rem_1fr] sm:gap-6">
              <dt className="font-semibold">{term}</dt>
              <dd className="text-sm">
                {def}
                {g.see && (
                  <>
                    {" "}
                    <Link to="/guide/$kind" params={{ kind: g.see }} className="font-semibold text-primary underline underline-offset-2">
                      {t("glossary.see", { name: kindName(t, g.see) })}
                    </Link>
                  </>
                )}
              </dd>
            </div>
          ))}
        </dl>
      )}
    </Page>
  );
}
