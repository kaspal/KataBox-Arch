// Lets the command palette add a component to the canvas: the canvas registers how to add
// at its viewport centre; a request made while the canvas isn't mounted waits for it.
import type { ComponentKind, Design, GroupKind } from "@/bindings";

/** Add a component (with the flavour picker when `pick`), a zone, a note, or a pattern's
 * template (with fresh ids, centred in the view, as one undo step). */
export type AddRequest = ({ kind: ComponentKind } | { meta: GroupKind | "note" } | { pattern: Design }) & { pick?: boolean };

let handler: ((req: AddRequest) => void) | null = null;
let pending: AddRequest | null = null;

export function registerCanvasAdd(h: (req: AddRequest) => void): () => void {
  handler = h;
  if (pending) {
    const req = pending;
    pending = null;
    h(req);
  }
  return () => {
    if (handler === h) handler = null;
  };
}

export function requestAdd(req: AddRequest) {
  if (handler) handler(req);
  else pending = req;
}
