// Reduced motion: the app's setting (`.reduce-motion` on <html>, see prefs.tsx) or the
// system's `prefers-reduced-motion`. Code that animates outside CSS (the packet layer)
// reads it here.
import { useSyncExternalStore } from "react";

const QUERY = "(prefers-reduced-motion: reduce)";

export function reducedMotion(): boolean {
  if (typeof document === "undefined") return false;
  if (document.documentElement.classList.contains("reduce-motion")) return true;
  return typeof window.matchMedia === "function" && window.matchMedia(QUERY).matches;
}

function subscribe(cb: () => void): () => void {
  const observer = new MutationObserver(cb);
  observer.observe(document.documentElement, { attributes: true, attributeFilter: ["class"] });
  const media = typeof window.matchMedia === "function" ? window.matchMedia(QUERY) : null;
  media?.addEventListener?.("change", cb);
  return () => {
    observer.disconnect();
    media?.removeEventListener?.("change", cb);
  };
}

export function useReducedMotion(): boolean {
  return useSyncExternalStore(subscribe, reducedMotion, () => false);
}
