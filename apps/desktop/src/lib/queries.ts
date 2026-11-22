// React Query hooks over `api`. Query keys live here so mutations can invalidate them.
import { useQuery } from "@tanstack/react-query";
import { api } from "@/lib/api";

export const queryKeys = {
  appInfo: ["app-info"] as const,
  registry: ["registry"] as const,
  recentFiles: ["recent-files"] as const,
};

export function useAppInfo() {
  return useQuery({ queryKey: queryKeys.appInfo, queryFn: api.appInfo, staleTime: Infinity });
}

/** The component registry from Rust: fetched once, never duplicated in TypeScript. */
export function useRegistry() {
  return useQuery({ queryKey: queryKeys.registry, queryFn: api.registry, staleTime: Infinity });
}

export function useRecentFiles() {
  return useQuery({ queryKey: queryKeys.recentFiles, queryFn: api.recentFiles });
}
