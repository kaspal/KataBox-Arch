// The Solve page is the Sandbox layout with a problem: the toolbar submits, Run plays the
// problem's tests, and the inspector gains a Brief tab. Pages without a problem (the Sandbox)
// provide nothing.
import { createContext, useContext } from "react";
import type { ProblemBrief } from "@/bindings";

export type ProblemMode = {
  id: string;
  brief: ProblemBrief;
  submitting: boolean;
  submit: () => void;
};

export const ProblemContext = createContext<ProblemMode | null>(null);

export function useProblemMode(): ProblemMode | null {
  return useContext(ProblemContext);
}
