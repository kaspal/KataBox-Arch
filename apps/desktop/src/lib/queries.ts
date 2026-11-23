// React Query hooks over `api`. Query keys live here so mutations can invalidate them.
import { useQuery } from "@tanstack/react-query";
import type { ComponentKind } from "@/bindings";
import { api } from "@/lib/api";
import { localDay } from "@/lib/dates";

export const queryKeys = {
  appInfo: ["app-info"] as const,
  registry: ["registry"] as const,
  recentFiles: ["recent-files"] as const,
  problems: ["problems"] as const,
  brief: (id: string) => ["problem-brief", id] as const,
  progress: ["progress"] as const,
  submissions: (id: string) => ["submissions", id] as const,
  samples: ["samples"] as const,
  guide: ["guide"] as const,
  guidePage: (kind: string) => ["guide", kind] as const,
  sample: (id: string) => ["sample", id] as const,
  estimateStatuses: ["estimate-statuses"] as const,
  flaws: ["flaws"] as const,
  outages: ["outages"] as const,
  outageStory: (id: string) => ["outage-story", id] as const,
  paths: ["paths"] as const,
  reviewsDue: ["reviews-due"] as const,
  quiz: (page: string) => ["quiz", page] as const,
};

export function useAppInfo() {
  return useQuery({ queryKey: queryKeys.appInfo, queryFn: api.appInfo, staleTime: Infinity });
}

/** The component registry from Rust: fetched once, never duplicated in TypeScript. */
export function useRegistry() {
  return useQuery({ queryKey: queryKeys.registry, queryFn: api.registry, staleTime: Infinity });
}

export function useProblems() {
  return useQuery({ queryKey: queryKeys.problems, queryFn: api.listProblems });
}

export function useProblemBrief(id: string) {
  return useQuery({ queryKey: queryKeys.brief(id), queryFn: () => api.problemBrief(id) });
}

export function useSubmissions(id: string) {
  return useQuery({ queryKey: queryKeys.submissions(id), queryFn: () => api.submissions(id) });
}

export function useGuide() {
  return useQuery({ queryKey: queryKeys.guide, queryFn: api.listGuide });
}

export function useGuidePage(kind: ComponentKind) {
  return useQuery({ queryKey: queryKeys.guidePage(kind), queryFn: () => api.guidePage(kind) });
}

export function useDecideFlows() {
  return useQuery({ queryKey: ["decide"], queryFn: api.decideFlows, staleTime: Infinity });
}

export function useGlossary() {
  return useQuery({ queryKey: ["glossary"], queryFn: api.glossary, staleTime: Infinity });
}

export function useNumbers() {
  return useQuery({ queryKey: ["numbers"], queryFn: api.numbers, staleTime: Infinity });
}

/** Every estimation drill (content: never changes while the app runs). */
export function useDrills() {
  return useQuery({ queryKey: ["drills"], queryFn: () => api.drills(), staleTime: Infinity });
}

export function useQuiz(page: string) {
  return useQuery({ queryKey: queryKeys.quiz(page), queryFn: () => api.quiz(page), staleTime: Infinity });
}

/** Today's due questions (the local date is read when the query runs). */
export function useReviewsDue() {
  return useQuery({ queryKey: queryKeys.reviewsDue, queryFn: () => api.reviewsDue(localDay()) });
}

export function usePaths() {
  return useQuery({ queryKey: queryKeys.paths, queryFn: api.learningPaths });
}

export function useFlaws() {
  return useQuery({ queryKey: queryKeys.flaws, queryFn: api.listFlaws });
}

export function useOutages() {
  return useQuery({ queryKey: queryKeys.outages, queryFn: api.listOutages });
}

export function useOutageStory(id: string) {
  return useQuery({ queryKey: queryKeys.outageStory(id), queryFn: () => api.outageStory(id) });
}

export function useEstimateStatuses() {
  return useQuery({ queryKey: queryKeys.estimateStatuses, queryFn: api.estimateStatuses });
}

export function usePatterns() {
  return useQuery({ queryKey: ["patterns"], queryFn: api.listPatterns });
}

export function usePattern(id: string) {
  return useQuery({ queryKey: ["patterns", id], queryFn: () => api.pattern(id) });
}

export function useConcept(kind: ComponentKind) {
  return useQuery({ queryKey: ["concept", kind], queryFn: () => api.concept(kind), staleTime: Infinity });
}

export function useSamples() {
  return useQuery({ queryKey: queryKeys.samples, queryFn: api.listSamples, staleTime: Infinity });
}

export function useSample(id: string) {
  return useQuery({ queryKey: queryKeys.sample(id), queryFn: () => api.sample(id), staleTime: Infinity });
}

export function useProgress() {
  return useQuery({ queryKey: queryKeys.progress, queryFn: () => api.progress(localDay()) });
}

export function useRecentFiles() {
  return useQuery({ queryKey: queryKeys.recentFiles, queryFn: api.recentFiles });
}
