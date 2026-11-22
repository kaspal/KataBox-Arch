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
