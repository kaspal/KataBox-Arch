// Content prose (statements, hints, editorials; the Guide from M3): Markdown with GitHub tables
// and lists. Raw HTML in content is dropped, never rendered (no rehype-raw; a test checks it).
import ReactMarkdown, { type Components } from "react-markdown";
import remarkGfm from "remark-gfm";
import { cn } from "@/lib/utils";

const components: Components = {
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

export function Markdown({ children, className }: { children: string; className?: string }) {
  return (
    <div className={cn("space-y-3 text-sm", className)} data-testid="markdown">
      <ReactMarkdown remarkPlugins={[remarkGfm]} skipHtml components={components}>
        {children}
      </ReactMarkdown>
    </div>
  );
}
