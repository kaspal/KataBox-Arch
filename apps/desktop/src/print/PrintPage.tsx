// The print layout (M7b.3): Guide entries, patterns and learning paths on paper, without
// the app around them. With `pdf=<token>` it prints itself to PDF through Rust once ready,
// then goes back; with `html=1` it saves itself as a print-ready HTML file. A "Preparing…"
// overlay shows meanwhile and never prints (review A8).
import { useEffect, useMemo, useRef, useState } from "react";
import { useQueries } from "@tanstack/react-query";
import { useRouter } from "@tanstack/react-router";
import { useTranslation } from "react-i18next";
import type { ComponentKind, GuidePage, PatternPage, PathView } from "@/bindings";
import { showNotice } from "@/app/dialogs";
import { Markdown } from "@/components/Markdown";
import { Button } from "@/components/ui/button";
import { api } from "@/lib/api";
import { describeError } from "@/lib/errors";
import { queryKeys, useGlossary, useGuide, usePaths, useRegistry } from "@/lib/queries";
import { flavourName, kindName } from "@/solve/registry";
import { designSvg, readPalette, type SceneNames } from "@/solve/export/scene";
import { offerHtml, printFileName, printHtml, type PrintWhat } from "@/print/printing";

export type PrintSearch = { what: PrintWhat; pdf?: number; html?: 1; back?: string; name?: string };

function List({ title, items }: { title: string; items: string[] }) {
  if (items.length === 0) return null;
  return (
    <section className="space-y-1">
      <h3 className="text-sm font-semibold">{title}</h3>
      <ul className="list-disc space-y-0.5 pl-5 text-sm">
        {items.map((x) => (
          <li key={x}>{x}</li>
        ))}
      </ul>
    </section>
  );
}

function EntrySection({ page }: { page: GuidePage }) {
  const { t, i18n } = useTranslation();
  const es = i18n.language.startsWith("es");
  const e = page.entry;
  const L = (en: string[], sp: string[]) => (es ? sp : en);
  return (
    <section className="print-section space-y-3" data-testid="print-entry">
      <h1 className="font-display text-3xl font-semibold">{kindName(t, e.kind)}</h1>
      <p className="text-base">{es ? e.one_liner_es : e.one_liner}</p>
      <Markdown glossary={false}>{es ? page.md_es : page.md_en}</Markdown>
      <div className="grid grid-cols-2 gap-4">
        <List title={t("guide.useWhen")} items={L(e.use_when, e.use_when_es)} />
        <List title={t("guide.skipWhen")} items={L(e.skip_when, e.skip_when_es)} />
        <List title={t("guide.gains")} items={L(e.gains, e.gains_es)} />
        <List title={t("guide.costs")} items={L(e.costs, e.costs_es)} />
      </div>
      <p className="text-sm">
        <span className="font-semibold">{t("print.formula")}: </span>
        {es ? e.formula_es : e.formula}
      </p>
      <p className="text-sm">
        <span className="font-semibold">{t("guide.interviewTip")}: </span>
        {es ? e.interview_tip_es : e.interview_tip}
      </p>
    </section>
  );
}

function PatternSection({ page }: { page: PatternPage }) {
  const { t, i18n } = useTranslation();
  const es = i18n.language.startsWith("es");
  const registry = useRegistry();
  const e = page.entry;
  const svg = useMemo(() => {
    if (!registry.data) return "";
    const names: SceneNames = {
      kind: (k) => kindName(t, k as never),
      flavour: (f) => flavourName(t, f as never),
      protocol: (p) => t(`protocol.${p}` as "protocol.http"),
      zone: (z) => t(`canvas.zone.${z}` as "canvas.zone.region"),
    };
    // Our own SVG, every text escaped (scene.ts): the template's picture.
    return designSvg(page.template, registry.data, names, readPalette());
  }, [page.template, registry.data, t]);
  return (
    <section className="print-section space-y-3" data-testid="print-pattern">
      <h1 className="font-display text-3xl font-semibold">{es ? e.title_es : e.title}</h1>
      <p className="text-base">{es ? e.one_liner_es : e.one_liner}</p>
      <Markdown glossary={false}>{es ? page.md_es : page.md_en}</Markdown>
      {svg && <figure className="print-figure max-w-full [&_svg]:h-auto [&_svg]:max-w-full" dangerouslySetInnerHTML={{ __html: svg }} />}
      <div className="grid grid-cols-2 gap-4">
        <List title={t("guide.useWhen")} items={es ? e.use_when_es : e.use_when} />
        <List title={t("guide.gains")} items={es ? e.gains_es : e.gains} />
        <List title={t("guide.costs")} items={es ? e.costs_es : e.costs} />
      </div>
      <p className="text-sm">
        <span className="font-semibold">{t("guide.inSimulator")}: </span>
        {es ? e.in_simulator_es : e.in_simulator}
      </p>
    </section>
  );
}

function PathHeader({ path }: { path: PathView }) {
  const { t, i18n } = useTranslation();
  const es = i18n.language.startsWith("es");
  return (
    <section className="print-section space-y-3" data-testid="print-path">
      <h1 className="font-display text-3xl font-semibold">{es ? path.title_es : path.title}</h1>
      <p>{es ? path.blurb_es : path.blurb}</p>
      <h2 className="text-base font-semibold">{t("print.pathItems")}</h2>
      <ol className="list-decimal space-y-0.5 pl-5 text-sm">
        {path.items.map((x) => (
          <li key={`${x.kind}:${x.target}`}>
            {t(`paths.item.${x.kind}` as "paths.item.guide", { name: (es ? x.title_es : x.title) ?? x.target })}
          </li>
        ))}
      </ol>
    </section>
  );
}

/** Every page `what` needs, loaded, or null while loading. */
function usePrintContent(what: PrintWhat) {
  const [kind, id] = what.includes(":") ? (what.split(":") as [string, string]) : [what, ""];
  const guide = useGuide();
  const paths = usePaths();
  const glossary = useGlossary();
  const path = kind === "path" ? paths.data?.find((p) => p.id === id) : undefined;
  const entryKinds: string[] =
    kind === "guide" ? (guide.data ?? []).map((g) => g.kind) : kind === "entry" ? [id] : kind === "path" ? (path?.items ?? []).filter((x) => x.kind === "guide").map((x) => x.target) : [];
  const patternIds: string[] = kind === "pattern" ? [id] : kind === "path" ? (path?.items ?? []).filter((x) => x.kind === "pattern").map((x) => x.target) : [];
  const entries = useQueries({
    queries: entryKinds.map((k) => ({ queryKey: queryKeys.guidePage(k as ComponentKind), queryFn: () => api.guidePage(k as ComponentKind) })),
  });
  const patterns = useQueries({
    queries: patternIds.map((p) => ({ queryKey: ["patterns", p], queryFn: () => api.pattern(p) })),
  });
  const waiting =
    (kind === "guide" && !guide.data) ||
    (kind === "path" && !paths.data) ||
    !glossary.data ||
    entries.some((q) => !q.data && !q.isError) ||
    patterns.some((q) => !q.data && !q.isError);
  const failed = entries.find((q) => q.isError)?.error ?? patterns.find((q) => q.isError)?.error ?? (kind === "path" && paths.data && !path ? new Error("no such path") : null);
  return {
    ready: !waiting,
    failed,
    path,
    entries: entries.flatMap((q) => (q.data ? [q.data] : [])),
    patterns: patterns.flatMap((q) => (q.data ? [q.data] : [])),
    glossary: kind === "guide" ? (glossary.data ?? []) : [],
  };
}

export function PrintPage({ search }: { search: PrintSearch }) {
  const { t, i18n } = useTranslation();
  const router = useRouter();
  const es = i18n.language.startsWith("es");
  const content = usePrintContent(search.what);
  const rootRef = useRef<HTMLDivElement>(null);
  const [ready, setReady] = useState(false);
  const cancelled = useRef(false);
  const started = useRef(false);
  const acting = search.pdf !== undefined || search.html === 1;
  const name = search.name ?? "KataBox Arch";
  const goBack = () => router.history.push(search.back && search.back.startsWith("/") ? search.back : "/guide");

  // Ready: everything loaded, webfonts in, then two frames so layout has settled.
  useEffect(() => {
    if (!content.ready || ready) return;
    let live = true;
    void (async () => {
      await document.fonts?.ready;
      await new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
      if (live) setReady(true);
    })();
    return () => {
      live = false;
    };
  }, [content.ready, ready]);

  useEffect(() => {
    if (!content.failed || !acting) return;
    goBack();
    showNotice(t("print.failed"), describeError(i18n, content.failed).summary);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [content.failed]);

  useEffect(() => {
    if (!ready || !acting || started.current) return;
    started.current = true;
    void (async () => {
      if (search.pdf !== undefined) {
        try {
          await api.printPdf(search.pdf);
          if (cancelled.current) return;
          goBack();
          showNotice(t("print.pdfSaved"));
        } catch (e) {
          if (cancelled.current) return;
          goBack();
          offerHtml(search.what, name, e);
        }
      } else if (rootRef.current) {
        try {
          const html = printHtml(rootRef.current, name, i18n.language);
          const saved = await api.saveExport("html", new TextEncoder().encode(html), printFileName(name, "html"));
          goBack();
          if (saved) showNotice(t("print.htmlSaved"));
        } catch (e) {
          goBack();
          showNotice(t("print.failed"), describeError(i18n, e).summary);
        }
      }
    })();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [ready]);

  return (
    <div className="print-doc min-h-full bg-white">
      {acting && (
        <div data-print-overlay className="fixed inset-0 z-50 flex items-center justify-center bg-black/40 print:hidden" data-testid="print-overlay">
          <div role="status" aria-live="polite" className="flex flex-col items-center gap-3 rounded-xl border bg-card p-6 text-card-foreground shadow-lg">
            <span className="size-6 animate-spin rounded-full border-2 border-primary border-t-transparent" aria-hidden />
            <p className="text-sm font-medium">{search.pdf !== undefined ? t("print.preparingPdf") : t("print.preparingHtml")}</p>
            <Button
              variant="outline"
              size="sm"
              onClick={() => {
                cancelled.current = true;
                goBack();
              }}
            >
              {t("common.cancel")}
            </Button>
          </div>
        </div>
      )}
      <div ref={rootRef} className="mx-auto max-w-3xl space-y-10 px-10 py-10 text-[#1f2421]" data-testid="print-root" data-print-ready={ready ? "1" : undefined}>
        <header className="print-only-header border-b pb-3 text-xs text-[#4a4f49]">KataBox Arch · {name}</header>
        {content.path && <PathHeader path={content.path} />}
        {content.entries.map((p) => (
          <EntrySection key={p.entry.kind} page={p} />
        ))}
        {content.patterns.map((p) => (
          <PatternSection key={p.entry.id} page={p} />
        ))}
        {content.glossary.length > 0 && (
          <section className="print-section space-y-3" data-testid="print-glossary">
            <h1 className="font-display text-3xl font-semibold">{t("glossary.title")}</h1>
            <dl className="space-y-2 text-sm">
              {content.glossary.map((g) => (
                <div key={g.id}>
                  <dt className="font-semibold">{es ? g.term_es : g.term}</dt>
                  <dd>{es ? g.definition_es : g.definition}</dd>
                </div>
              ))}
            </dl>
          </section>
        )}
      </div>
    </div>
  );
}
