// Typed user settings: user.db through get/set_setting is the source of truth (the keys
// are allow-listed in Rust, `ka-app/src/settings.rs`); `mirror` keeps a localStorage copy
// so layout-like settings apply on first paint.
import { useCallback } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import type { JsonValue } from "@/bindings";
import { api } from "@/lib/api";

const MIRROR_PREFIX = "katabox-arch.setting.";

function readMirror(key: string): JsonValue | undefined {
  try {
    const raw = localStorage.getItem(MIRROR_PREFIX + key);
    return raw === null ? undefined : (JSON.parse(raw) as JsonValue);
  } catch {
    return undefined;
  }
}

function writeMirror(key: string, value: JsonValue) {
  try {
    localStorage.setItem(MIRROR_PREFIX + key, JSON.stringify(value));
  } catch {
    // Storage unavailable: user.db still has it.
  }
}

export const settingKey = (key: string) => ["setting", key] as const;

/**
 * A setting parsed by `parse` (which returns undefined for invalid values, so the
 * fallback applies). Returns the value and a setter that saves it.
 */
export function useSetting<T>(
  key: string,
  fallback: T,
  parse: (v: JsonValue | null) => T | undefined,
  opts: { mirror?: boolean } = {},
): [T, (value: T & JsonValue) => void] {
  const queryClient = useQueryClient();
  const query = useQuery({
    queryKey: settingKey(key),
    queryFn: () => api.getSetting(key),
    staleTime: Infinity,
    placeholderData: opts.mirror ? (readMirror(key) ?? undefined) : undefined,
  });
  const parsed = query.data === undefined ? undefined : parse(query.data);
  const value = parsed ?? fallback;
  const set = useCallback(
    (next: T & JsonValue) => {
      queryClient.setQueryData(settingKey(key), next);
      if (opts.mirror) writeMirror(key, next);
      void api.setSetting(key, next).catch(() => {});
    },
    [key, opts.mirror, queryClient],
  );
  return [value, set];
}

// --- Sandbox ----------------------------------------------------------------------------

/** Panel sizes (percent) of the Sandbox: library | canvas | inspector. */
export type SandboxLayout = { [id: string]: number };

export const parseLayout = (v: JsonValue | null): SandboxLayout | undefined => {
  if (typeof v !== "object" || v === null || Array.isArray(v)) return undefined;
  const entries = Object.entries(v);
  return entries.every(([, n]) => typeof n === "number" && n >= 0 && n <= 100)
    ? (Object.fromEntries(entries) as SandboxLayout)
    : undefined;
};

export const parseBool = (v: JsonValue | null): boolean | undefined => (typeof v === "boolean" ? v : undefined);

/** Snap cards to the 8 px grid (hold Alt while dragging to place freely). */
export function useSnap() {
  return useSetting<boolean>("canvas.snap", true, parseBool, { mirror: true });
}
