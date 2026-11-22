// Content prose (statements, hints, editorials, the Guide, patterns): Markdown with GitHub
// tables and lists. Raw HTML in content is dropped, never rendered (no rehype-raw; a test
// checks it). The first use of each glossary term links to the glossary, with its
// definition in a tooltip on hover or keyboard focus. Text from a problem pack (M6.4,
// `trust="pack"`) shows images as their alt text: a pack can't make the page load anything.
import { useMemo } from "react";
import { useTranslation } from "react-i18next";
import { Link } from "@tanstack/react-router";
import ReactMarkdown, { type Components } from "react-markdown";
import remarkGfm from "remark-gfm";
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from "@/components/ui/tooltip";
import { remarkGlossary, type GlossaryNeedle } from "@/lib/glossaryRemark";
import { useGlossary } from "@/lib/queries";
import { cn } from "@/lib/utils";

const base: Components = {
  h1: ({ children }) => <h2 className="font-display text-2xl font-semibold">{children}</h2>,
  h2: ({ children }) => <h3 className="pt-2 text-base font-semibold">{children}</h3>,
  h3: ({ children }) => <h4 className="text-sm font-semibold">{children}</h4>,
  p: ({ children }) => <p className="leading-relaxed">{children}</p>,
  ul: ({ children }) => <ul className="list-disc space-y-1 pl-5">{children}</ul>,
  ol: ({ children }) => <ol className="list-decimal space-y-1 pl-5">{children}</ol>,
  code: ({ children }) => <code className="rounded bg-muted px-1 font-mono text-[0.85em]">{children}</code>,
  table: ({ children }) => (
    <div className="overflow-x-auto">
      <table className="w-full border-collapse text-left text-xs">{children}</table>
    </div>
  ),
  th: ({ children }) => <th className="border-b px-2 py-1 font-semibold">{children}</th>,
  td: ({ children }) => <td className="border-b px-2 py-1">{children}</td>,
  // Links in content open nowhere (offline app, no navigation away): shown as text.
  a: ({ children }) => <span className="underline decoration-dotted">{children}</span>,
};

function GlossaryTerm({ id, definition, children }: { id: string; definition: string; children: React.ReactNode }) {
  return (
    <TooltipProvider delayDuration={300}>
      <Tooltip>
        <TooltipTrigger asChild>
          <Link
            to="/glossary"
            search={{ term: id }}
            data-testid="glossary-term"
            className="rounded-sm underline decoration-dotted decoration-1 underline-offset-[3px] hover:decoration-solid focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
          >
            <dfn className="not-italic">{children}</dfn>
          </Link>
        </TooltipTrigger>
        <TooltipContent className="max-w-xs text-xs leading-snug">{definition}</TooltipContent>
      </Tooltip>
    </TooltipProvider>
  );
}

export function Markdown({
  children,
  className,
  glossary = true,
  trust = "app",
}: {
  children: string;
  className?: string;
  glossary?: boolean;
  /** Where the text came from: built-in content, or an imported pack. */
  trust?: "app" | "pack";
}) {
  const { i18n } = useTranslation();
  const terms = useGlossary();
  const es = i18n.language.startsWith("es");
  const { plugins, components } = useMemo(() => {
    const list = glossary ? (terms.data ?? []) : [];
    const needles: GlossaryNeedle[] = list.map((g) => ({ id: g.id, text: es ? g.term_es : g.term }));
    const defs = new Map(list.map((g) => [g.id, es ? g.definition_es : g.definition]));
    const comps: Components = {
      ...base,
      ...(trust === "pack"
        ? { img: ({ alt }: { alt?: string }) => <span data-testid="inert-image">{alt ? `[${alt}]` : ""}</span> }
        : {}),
      dfn: ({ node, children: inner }) => {
        const id = String(node?.properties?.dataTerm ?? "");
        const def = defs.get(id);
        return def ? (
          <GlossaryTerm id={id} definition={def}>
            {inner}
          </GlossaryTerm>
        ) : (
          <>{inner}</>
        );
      },
    };
    return { plugins: needles.length > 0 ? [remarkGfm, remarkGlossary(needles)] : [remarkGfm], components: comps };
  }, [glossary, terms.data, es, trust]);
  return (
    <div className={cn("space-y-3 text-sm", className)} data-testid="markdown">
      <ReactMarkdown remarkPlugins={plugins} skipHtml components={components}>
        {children}
      </ReactMarkdown>
    </div>
  );
}
