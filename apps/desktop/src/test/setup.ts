import "@testing-library/jest-dom/vitest";
import { afterEach, vi } from "vitest";
import { cleanup, configure } from "@testing-library/react";
import i18n from "i18next";
import { initI18n } from "@/i18n";
import { resetDialogs } from "@/app/dialogs";
import { resetComposeSummary } from "@/solve/ComposeSummary";
import { resetAnimation } from "@/solve/export/animationStore";
import { resetPrintFallback } from "@/print/printing";
import { useEditor } from "@/solve/store";

// jsdom lacks a few browser APIs that Radix, cmdk and the virtualizer use.
class ResizeObserverStub {
  observe() {}
  unobserve() {}
  disconnect() {}
}
globalThis.ResizeObserver ??= ResizeObserverStub as unknown as typeof ResizeObserver;
Element.prototype.scrollIntoView ??= function () {};
Element.prototype.hasPointerCapture ??= () => false;
Element.prototype.releasePointerCapture ??= function () {};
Element.prototype.scrollTo ??= function () {} as typeof Element.prototype.scrollTo;
// Text ranges (cmdk, Radix).
Range.prototype.getClientRects ??= () => ({ length: 0, item: () => null, [Symbol.iterator]: [][Symbol.iterator] }) as unknown as DOMRectList;
Range.prototype.getBoundingClientRect ??= () => new DOMRect(0, 0, 0, 0);
document.elementFromPoint ??= () => null;
// jsdom has no layout: give elements a size so React Flow has a viewport.
Object.defineProperty(HTMLElement.prototype, "clientHeight", { configurable: true, get: () => 600 });
Object.defineProperty(HTMLElement.prototype, "clientWidth", { configurable: true, get: () => 800 });

/** `prefers-color-scheme: dark` answer for the tests (default light). */
export const media = { dark: false, listeners: new Set<() => void>() };
window.matchMedia = vi.fn((query: string) => ({
  get matches() {
    return query.includes("dark") ? media.dark : false;
  },
  media: query,
  onchange: null,
  // Only the colour-scheme query ever changes in tests, so only its listeners are kept
  // (a browser notifies just the listeners of the query that changed; some libraries
  // listen to "print").
  addEventListener: (_: string, cb: () => void) => {
    if (query.includes("prefers-color-scheme")) media.listeners.add(cb);
  },
  removeEventListener: (_: string, cb: () => void) => media.listeners.delete(cb),
  addListener: () => {},
  removeListener: () => {},
  dispatchEvent: () => false,
})) as unknown as typeof window.matchMedia;

initI18n("en");

// A missing translation key fails the test that rendered it (review F1): typed keys catch
// most mistakes at compile time, but keys built at run time (plurals, `as` casts) don't.
const missingKeys: string[] = [];
i18n.options.saveMissing = true;
i18n.options.missingKeyHandler = (_lngs, _ns, key) => {
  missingKeys.push(key);
};
afterEach(() => {
  const found = missingKeys.splice(0);
  if (found.length > 0) throw new Error(`missing i18n keys: ${[...new Set(found)].join(", ")}`);
});

// One generous default for every findBy*/waitFor (Testing Library's own is 1 s). The
// lazy Sandbox route can take a few seconds
// when the suite runs right after a heavy cargo build; a per-call timeout here and
// there is what made those tests flaky.
configure({ asyncUtilTimeout: 10_000 });

vi.mock("@/components/ui/resizable", () => import("@/test/resizableStub"));

afterEach(() => {
  cleanup();
  // Module-level stores outlive a render: start each test with a fresh design.
  resetDialogs();
  resetComposeSummary();
  resetAnimation();
  resetPrintFallback();
  useEditor.getState().newDesign("Untitled design");
  localStorage.clear();
  document.documentElement.classList.remove("dark");
  media.dark = false;
  media.listeners.clear();
  initI18n("en");
});

// React Flow measures nodes with ResizeObserver and DOMMatrix.
globalThis.DOMMatrixReadOnly ??= class {
  m22: number;
  constructor(transform?: string) {
    const scale = transform?.match(/scale\(([1-9.])\)/)?.[1];
    this.m22 = scale !== undefined ? +scale : 1;
  }
} as unknown as typeof DOMMatrixReadOnly;
