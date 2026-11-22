// React Query hooks over `api`. Query keys live here so mutations can invalidate them.
import { useQuery } from "@tanstack/react-query";
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

export function useProgress() {
  return useQuery({ queryKey: queryKeys.progress, queryFn: () => api.progress(localDay()) });
}

export function useRecentFiles() {
  return useQuery({ queryKey: queryKeys.recentFiles, queryFn: api.recentFiles });
}
