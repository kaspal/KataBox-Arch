// A remark plugin that marks the first use of each glossary term in a document (M3.7's
// deferred hover definitions). The term becomes a `<dfn data-term="id">` element, which the
// Markdown component renders as a link to the glossary with the definition in a tooltip.
// Headings, links, code and table headers are left alone; matching ignores case and needs
// whole words (letters and digits around a match don't count).

/** The tree shapes this plugin touches (a minimal mdast subset). */
type MdNode = {
  type: string;
  value?: string;
  children?: MdNode[];
  data?: { hName?: string; hProperties?: Record<string, string> };
};

export type GlossaryNeedle = { id: string; text: string };

const SKIP = new Set(["heading", "link", "linkReference", "inlineCode", "code", "glossaryTerm", "tableHead", "definition", "html"]);

function escape(s: string): string {
  return s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

/** Compile the terms once: longest first, so "cache hit rate" wins over "cache". */
export function compileNeedles(terms: GlossaryNeedle[]): { id: string; re: RegExp }[] {
  return [...terms]
    .filter((t) => t.text.trim().length > 1)
    .sort((a, b) => b.text.length - a.text.length)
    .map((t) => ({ id: t.id, re: new RegExp(`(?<![\\p{L}\\p{N}])${escape(t.text.trim())}(?![\\p{L}\\p{N}])`, "iu") }));
}

/** Split one text value at the earliest unused term, repeatedly. */
export function splitText(value: string, needles: { id: string; re: RegExp }[], used: Set<string>): MdNode[] {
  const out: MdNode[] = [];
  let rest = value;
  for (;;) {
    let best: { id: string; index: number; length: number } | null = null;
    for (const n of needles) {
      if (used.has(n.id)) continue;
      const m = n.re.exec(rest);
      if (m && (best === null || m.index < best.index)) best = { id: n.id, index: m.index, length: m[0].length };
    }
    if (!best) break;
    if (best.index > 0) out.push({ type: "text", value: rest.slice(0, best.index) });
    out.push({
      type: "glossaryTerm",
      data: { hName: "dfn", hProperties: { dataTerm: best.id } },
      children: [{ type: "text", value: rest.slice(best.index, best.index + best.length) }],
    });
    used.add(best.id);
    rest = rest.slice(best.index + best.length);
  }
  if (rest) out.push({ type: "text", value: rest });
  return out;
}

export function remarkGlossary(terms: GlossaryNeedle[]) {
  const needles = compileNeedles(terms);
  return () => (tree: MdNode) => {
    if (needles.length === 0) return;
    const used = new Set<string>();
    const walk = (node: MdNode) => {
      if (!node.children || SKIP.has(node.type)) return;
      const next: MdNode[] = [];
      for (const child of node.children) {
        if (child.type === "text" && child.value) next.push(...splitText(child.value, needles, used));
        else {
          walk(child);
          next.push(child);
        }
      }
      node.children = next;
    };
    walk(tree);
  };
}
