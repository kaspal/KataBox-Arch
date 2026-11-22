// What the canvas's nodes and edges need besides their own data.
import { createContext, useContext } from "react";
import type { Registry } from "@/bindings";
import type { Wiring } from "@/solve/wiring";

export type CanvasContextValue = { registry: Registry; wiring: Wiring };

export const CanvasContext = createContext<CanvasContextValue | null>(null);

export function useCanvas(): CanvasContextValue {
  const ctx = useContext(CanvasContext);
  if (!ctx) throw new Error("useCanvas outside the canvas");
  return ctx;
}
