// Printable Guide (M7b.3): how a page asks for a PDF or a print-ready HTML file, and the
// self-contained HTML the fallback saves.
//
// PDF (Windows): Rust's Save dialog keeps the path under a one-shot token; the app moves to
// `/print?what=…&pdf=<token>&back=<where it was>`, the print page says when it's ready, then
// asks Rust to print the webview to that path, and goes back. On any error it goes back and
// offers the HTML instead (review A8).
import { create } from "zustand";

/** What to print: the whole Guide, one entry, one pattern, or one learning path. */
export type PrintWhat = "guide" | `entry:${string}` | `pattern:${string}` | `path:${string}`;

export function validWhat(s: unknown): s is PrintWhat {
  return typeof s === "string" && /^(guide|(entry|pattern|path):[a-z0-9_-]{1,64})$/.test(s);
}

/** The "PDF didn't work, save HTML instead?" question, shown after going back. */
export const usePrintFallback = create<{ what: PrintWhat | null; name: string; error: unknown }>(() => ({
  what: null,
  name: "",
  error: null,
}));

export function offerHtml(what: PrintWhat, name: string, error: unknown) {
  usePrintFallback.setState({ what, name, error });
}

export function resetPrintFallback() {
  usePrintFallback.setState({ what: null, name: "", error: null });
}

/** A file name without characters Windows refuses. */
export function printFileName(name: string, ext: "pdf" | "html"): string {
  const clean = name.replace(/[<>:"/\\|?*\u0000-\u001f]/g, "").trim() || "KataBox Arch";
  return `${clean.slice(0, 80)}.${ext}`;
}

const esc = (s: string) => s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");

/**
 * The print page as one self-contained HTML document: the printed content, the page's CSS
 * inlined (font files left out: fonts are named, not embedded), and nothing that runs: no
 * scripts, no event handlers, no buttons.
 */
export function printHtml(root: HTMLElement, title: string, lang: string, sheets: Iterable<CSSStyleSheet> = Array.from(document.styleSheets)): string {
  const rules: string[] = [];
  for (const sheet of sheets) {
    let list: CSSRuleList;
    try {
      list = sheet.cssRules;
    } catch {
      continue;
    }
    for (const r of Array.from(list)) {
      if (r.cssText.startsWith("@font-face") || r.cssText.startsWith("@import")) continue;
      rules.push(r.cssText);
    }
  }
  const clone = root.cloneNode(true) as HTMLElement;
  clone.querySelectorAll("script, button, [data-print-overlay], iframe, object, embed").forEach((n) => n.remove());
  for (const el of [clone, ...Array.from(clone.querySelectorAll("*"))]) {
    for (const a of Array.from(el.attributes)) {
      if (/^on/i.test(a.name) || /^\s*javascript:/i.test(a.value)) el.removeAttribute(a.name);
    }
  }
  // No "<" survives in the CSS (written as its CSS escape), so nothing can close the style.
  const css = rules.join("\n").replace(/</g, "\\3c ");
  return `<!doctype html>\n<html lang="${esc(lang)}">\n<head>\n<meta charset="utf-8">\n<meta name="viewport" content="width=device-width, initial-scale=1">\n<title>${esc(title)}</title>\n<style>\n${css}\n</style>\n</head>\n<body>\n${clone.outerHTML}\n</body>\n</html>\n`;
}
