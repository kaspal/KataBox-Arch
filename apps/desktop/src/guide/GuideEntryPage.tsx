// /guide/$kind: one Component Guide entry (guide-entry mockup). What it does, when to use it
// or skip it, how the simulator models it (numbers read from the registry, one column per
// flavour), the flavours compared, pitfalls, trade-offs, practice, related kinds, an
// interview tip and the vendor sources. `?section=sim` scrolls to "In the simulator".
import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { useQueryClient } from "@tanstack/react-query";
import { Link, useParams, useSearch } from "@tanstack/react-router";
import { BookOpen, Check, Circle, Lightbulb, PlayCircle, X } from "lucide-react";
import type { ComponentKind, GuidePage, KindSpec, Registry } from "@/bindings";
import { useReportError } from "@/app/dialogs";
import { Markdown } from "@/components/Markdown";
import { EmptyState, Page } from "@/components/Page";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { api } from "@/lib/api";
import { describeError } from "@/lib/errors";
import { queryKeys, useConcept, useGuidePage, useProblems, useRegistry } from "@/lib/queries";
import { DifficultyBadge, useLocalized } from "@/pages/Problems";
import { GlyphTile } from "@/solve/icons";
import { categoryName, flavourName, kindName, kindSpec } from "@/solve/registry";
import { simLabel, simValue } from "@/guide/values";
import { QuizBlock } from "@/practice/QuizBlock";
import { RelatedInKatabox } from "@/guide/RelatedInKatabox";
import { Player } from "@/tour/Player";
import { DesignThumb } from "@/tour/SamplesPage";

function Card({ title, id, children, className }: { title: string; id?: string; children: React.ReactNode; className?: string }) {
  return (
    <section id={id} aria-labelledby={id ? `${id}-h` : undefined} className={`scroll-mt-4 space-y-3 rounded-xl border bg-card p-5 ${className ?? ""}`}>
      <h2 id={id ? `${id}-h` : undefined} className="font-display text-2xl font-semibold">
        {title}
      </h2>
      {children}
    </section>
  );
}

function SimTable({ page, spec }: { page: GuidePage; spec: KindSpec }) {
  const { t, i18n } = useTranslation();
  const es = i18n.language.startsWith("es");
  const flavours = spec.flavours.map((f) => f.flavour);
  return (
    <div className="overflow-x-auto">
      <table className="w-full text-sm" data-testid="sim-table">
        <thead>
          <tr className="border-b text-left text-xs text-muted-foreground">
            <th className="py-2 pr-3 font-medium">{t("guide.value")}</th>
            {flavours.map((f) => (
              <th key={f} className="py-2 pr-3 font-medium">
                {flavourName(t, f)}
              </th>
            ))}
            <th className="py-2 font-medium">{t("guide.whatItDoes")}</th>
          </tr>
        </thead>
        <tbody>
          {page.entry.sim.map((row, i) => (
            <tr key={i} className="border-b align-top last:border-0">
              <th scope="row" className="py-2 pr-3 text-left font-semibold">
                {simLabel(t, row)}
              </th>
              {flavours.map((f) => (
                <td key={f} className="py-2 pr-3 font-mono whitespace-nowrap tabular-nums">
                  {simValue(t, i18n.language, spec, f, row)}
                </td>
              ))}
              <td className="py-2 text-muted-foreground">{es ? row.note_es : row.note}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

function Entry({ page, registry }: { page: GuidePage; registry: Registry }) {
  const { t, i18n } = useTranslation();
  const loc = useLocalized();
  const report = useReportError();
  const qc = useQueryClient();
  const problems = useProblems();
  const { section } = useSearch({ from: "/guide/$kind" });
  const simRef = useRef<HTMLDivElement>(null);
  const concept = useConcept(page.entry.kind);
  const [playing, setPlaying] = useState(false);
  const e = page.entry;
  const es = i18n.language.startsWith("es");
  const spec = kindSpec(registry, e.kind);
  const name = kindName(t, e.kind);
  const L = <T,>(en: T, esv: T) => (es ? esv : en);

  useEffect(() => {
    if (section === "sim") simRef.current?.scrollIntoView({ block: "start" });
  }, [section, e.kind]);

  const toggleRead = async () => {
    try {
      await api.setGuideRead(e.kind, !page.read);
      await Promise.all([qc.invalidateQueries({ queryKey: queryKeys.guidePage(e.kind) }), qc.invalidateQueries({ queryKey: queryKeys.guide })]);
    } catch (err) {
      report(err);
    }
  };

  return (
    <div className="mx-auto max-w-6xl space-y-6 px-8 py-8" data-testid="guide-entry">
      <nav aria-label={t("guide.breadcrumb")} className="text-sm">
        <Link to="/guide" className="underline underline-offset-4">
          {t("guide.title")}
        </Link>
        <span aria-hidden className="mx-2 text-muted-foreground">
          /
        </span>
        <span className="text-muted-foreground">{categoryName(t, spec.category)}</span>
        <span aria-hidden className="mx-2 text-muted-foreground">
          /
        </span>
        <span>{name}</span>
      </nav>
      <header className="flex flex-wrap items-start justify-between gap-4">
        <div className="flex items-center gap-4">
          <GlyphTile icon={spec.icon} category={spec.category} className="size-16 rounded-2xl [&_svg]:size-8" />
          <div>
            <h1 className="font-display text-5xl font-semibold">{name}</h1>
            <p className="text-lg text-muted-foreground">{L(e.one_liner, e.one_liner_es)}</p>
          </div>
        </div>
        <div className="flex flex-col items-end gap-3">
          <ul className="flex flex-wrap justify-end gap-1.5" aria-label={t("library.flavours")}>
            {spec.flavours.map((f) => (
              <li key={f.flavour} className="rounded-full border bg-card px-2.5 py-0.5 text-xs font-medium">
                {flavourName(t, f.flavour)}
              </li>
            ))}
          </ul>
          <Button variant={page.read ? "outline" : "default"} className="gap-2" onClick={() => void toggleRead()} aria-pressed={page.read}>
            {page.read ? <Check aria-hidden className="size-4" /> : <Circle aria-hidden className="size-4" />}
            {page.read ? t("guide.markedRead") : t("guide.markRead")}
          </Button>
        </div>
      </header>

      {playing && concept.data && (
        <section className="h-[38rem] overflow-hidden rounded-xl border bg-card" aria-label={t("guide.conceptCard")} data-testid="concept-player">
          <Player
            story={{
              title: L(concept.data.def.title, concept.data.def.title_es),
              problemId: concept.data.def.problem ?? null,
              steps: concept.data.def.steps,
              snapshots: concept.data.snapshots,
            }}
            registry={registry}
            onLastStep={() => {
              void api.setConceptPlayed(e.kind).then(() => qc.invalidateQueries({ queryKey: queryKeys.paths }));
            }}
            header={
              <>
                <h2 className="font-display text-xl font-semibold">{t("guide.conceptCard")}</h2>
                <Button variant="ghost" size="sm" className="gap-1" onClick={() => setPlaying(false)}>
                  <X aria-hidden className="size-4" />
                  {t("common.close")}
                </Button>
              </>
            }
          />
        </section>
      )}
      <div className="grid gap-6 lg:grid-cols-[minmax(0,1fr)_22rem]">
        <div className="space-y-6">
          <Card title={t("guide.whatItDoes")}>
            <Markdown>{L(page.md_en, page.md_es)}</Markdown>
          </Card>
          <div className="grid gap-6 sm:grid-cols-2">
            <section className="space-y-2 rounded-xl border bg-card p-5" aria-labelledby="use-when">
              <h2 id="use-when" className="font-semibold text-green">
                {t("guide.useWhen")}
              </h2>
              <ul className="space-y-2 text-sm">
                {L(e.use_when, e.use_when_es).map((s) => (
                  <li key={s} className="flex gap-2">
                    <Check aria-hidden className="mt-0.5 size-4 shrink-0 text-green" />
                    {s}
                  </li>
                ))}
              </ul>
            </section>
            <section className="space-y-2 rounded-xl border bg-card p-5" aria-labelledby="skip-when">
              <h2 id="skip-when" className="font-semibold text-destructive">
                {t("guide.skipWhen")}
              </h2>
              <ul className="space-y-2 text-sm">
                {L(e.skip_when, e.skip_when_es).map((s) => (
                  <li key={s} className="flex gap-2">
                    <X aria-hidden className="mt-0.5 size-4 shrink-0 text-destructive" />
                    {s}
                  </li>
                ))}
              </ul>
            </section>
          </div>
          <div ref={simRef}>
            <Card title={t("guide.inSimulator")} id="sim">
              <SimTable page={page} spec={spec} />
              <pre className="rounded-md bg-muted p-3 font-mono text-xs whitespace-pre-wrap" data-testid="formula">
                {L(e.formula, e.formula_es)}
              </pre>
              <p className="text-xs text-muted-foreground">{t("guide.simNote")}</p>
            </Card>
          </div>
          <Card title={t("guide.pitfalls")}>
            <dl className="divide-y">
              {e.pitfalls.map((p) => (
                <div key={p.title} className="grid gap-1 py-3 sm:grid-cols-[12rem_1fr] sm:gap-4">
                  <dt className="font-semibold">{L(p.title, p.title_es)}</dt>
                  <dd className="space-y-1 text-sm">
                    <p>{L(p.body, p.body_es)}</p>
                    <p>
                      <span className="font-semibold text-green">{t("guide.fix")}</span> {L(p.fix, p.fix_es)}
                    </p>
                    {p.seen_in && (
                      <p className="font-mono text-xs text-muted-foreground">
                        {t("guide.seenIn")}{" "}
                        <Link to="/problems/$id" params={{ id: p.seen_in }} className="underline underline-offset-2">
                          {(() => {
                            const s = problems.data?.find((x) => x.id === p.seen_in);
                            return s ? loc.title(s) : p.seen_in;
                          })()}
                        </Link>
                      </p>
                    )}
                  </dd>
                </div>
              ))}
            </dl>
          </Card>
          <Card title={t("guide.tradeoffs")}>
            <div className="grid gap-4 sm:grid-cols-2">
              <div>
                <h3 className="mb-1 text-sm font-semibold text-green">{t("guide.gains")}</h3>
                <ul className="list-disc space-y-1 pl-5 text-sm">
                  {L(e.gains, e.gains_es).map((s) => (
                    <li key={s}>{s}</li>
                  ))}
                </ul>
              </div>
              <div>
                <h3 className="mb-1 text-sm font-semibold text-destructive">{t("guide.costs")}</h3>
                <ul className="list-disc space-y-1 pl-5 text-sm">
                  {L(e.costs, e.costs_es).map((s) => (
                    <li key={s}>{s}</li>
                  ))}
                </ul>
              </div>
            </div>
          </Card>
        </div>

        <aside className="space-y-6">
          {concept.data && !playing && (
            <section className="space-y-3 rounded-xl border bg-card p-5" aria-labelledby="concept-card" data-testid="concept-card">
              <h2 id="concept-card" className="text-xs font-semibold tracking-wide uppercase">
                {t("guide.conceptCard")}
              </h2>
              {concept.data.snapshots[concept.data.def.steps[0].snapshot] && (
                <DesignThumb design={concept.data.snapshots[concept.data.def.steps[0].snapshot]!} registry={registry} label={t("guide.conceptCard")} />
              )}
              <Button className="w-full gap-2" onClick={() => setPlaying(true)}>
                <PlayCircle aria-hidden className="size-4" />
                {t("guide.playConcept", { count: concept.data.def.steps.length })}
              </Button>
            </section>
          )}
          {e.flavours_compared && (
            <Card title={t("guide.flavoursCompared")}>
              <div className="overflow-x-auto">
                <table className="w-full text-sm" data-testid="flavours-compared">
                  <thead>
                    <tr className="border-b text-left">
                      <td />
                      {e.flavours_compared.flavours.map((f) => (
                        <th key={f} className="py-1.5 pr-2 font-semibold">
                          {flavourName(t, f)}
                        </th>
                      ))}
                    </tr>
                  </thead>
                  <tbody>
                    {e.flavours_compared.rows.map((r) => (
                      <tr key={r.label} className="border-b align-top last:border-0">
                        <th scope="row" className="py-1.5 pr-2 text-left font-normal text-muted-foreground">
                          {L(r.label, r.label_es)}
                        </th>
                        {L(r.cells, r.cells_es).map((c, i) => (
                          <td key={i} className="py-1.5 pr-2">
                            {c}
                          </td>
                        ))}
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </Card>
          )}
          {(e.practice ?? []).length > 0 && (
            <Card title={t("guide.practice")}>
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
            </Card>
          )}
          <section className="space-y-2 rounded-xl border bg-card p-5" aria-labelledby="works-with">
            <h2 id="works-with" className="text-xs font-semibold tracking-wide uppercase">
              {t("guide.worksWith")}
            </h2>
            <ul className="flex flex-wrap gap-2">
              {e.works_with.map((k: ComponentKind) => {
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
          <section className="flex gap-3 rounded-xl border bg-card p-5" aria-labelledby="tip" data-testid="interview-tip">
            <Lightbulb aria-hidden className="mt-1 size-5 shrink-0 text-[var(--amber-ink)]" />
            <div>
              <h2 id="tip" className="text-xs font-semibold tracking-wide uppercase">
                {t("guide.interviewTip")}
              </h2>
              <p className="font-display text-lg leading-snug">{L(e.interview_tip, e.interview_tip_es)}</p>
            </div>
          </section>
          <section className="space-y-2 rounded-xl border bg-card p-5 text-sm" aria-labelledby="sources">
            <h2 id="sources" className="text-xs font-semibold tracking-wide uppercase">
              {t("guide.sources")}
            </h2>
            <ul className="space-y-1">
              {e.sources.map((s) => (
                <li key={s.url}>
                  <span className="font-medium">{flavourName(t, s.flavour)}:</span> {s.title}
                  <span className="block font-mono text-xs break-all text-muted-foreground select-all">{s.url}</span>
                </li>
              ))}
            </ul>
            {e.sources.some((s) => !s.checked) && <p className="text-xs text-muted-foreground">{t("guide.sourcesUnchecked")}</p>}
          </section>
        </aside>
      </div>
      <QuizBlock page={`guide:${e.kind}`} />
      <RelatedInKatabox target={`guide:${e.kind}`} />
    </div>
  );
}

export function GuideEntryPage() {
  const { t, i18n } = useTranslation();
  const { kind } = useParams({ from: "/guide/$kind" });
  const page = useGuidePage(kind as ComponentKind);
  const registry = useRegistry();
  if (page.error) {
    return (
      <Page title={t("guide.title")}>
        <EmptyState icon={BookOpen} title={t("guide.unavailable")} body={describeError(i18n, page.error).summary} />
      </Page>
    );
  }
  if (!page.data || !registry.data) return <Skeleton className="m-8 h-96" />;
  return <Entry page={page.data} registry={registry.data} />;
}
