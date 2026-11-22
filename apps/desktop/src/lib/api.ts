// The UI's only door to the backend. Wraps the generated tauri-specta commands: unwraps
// `{status, data | error}` and throws the typed AppError. Components use the hooks in
// `queries.ts` (or call `api` for actions); tests mock this module (`test/mockApi.ts`).
import { listen } from "@tauri-apps/api/event";
import {
  commands,
  type AppError,
  type AppInfo,
  type Design,
  type DesignIssue,
  type JsonValue,
  type OpenedDesign,
  type RecentEntry,
  type Registry,
  type Result,
} from "@/bindings";

export async function call<T>(p: Promise<Result<T, AppError>>): Promise<T> {
  const r = await p;
  if (r.status === "error") throw r.error;
  return r.data;
}

/** The event Rust emits when a close was held back by unsaved changes. */
const CLOSE_REQUESTED = "close-requested";

export const api = {
  appInfo: (): Promise<AppInfo> => call(commands.appInfo()),
  registry: (): Promise<Registry> => call(commands.registry()),
  getSetting: (key: string): Promise<JsonValue | null> => call(commands.getSetting(key)),
  setSetting: (key: string, value: JsonValue): Promise<null> => call(commands.setSetting(key, value)),

  recentFiles: (): Promise<RecentEntry[]> => call(commands.recentFiles()),
  forgetRecent: (path: string): Promise<null> => call(commands.forgetRecent(path)),
  openDesignDialog: (): Promise<OpenedDesign | null> => call(commands.openDesignDialog()),
  openRecent: (path: string): Promise<OpenedDesign> => call(commands.openRecent(path)),
  saveDesign: (path: string, design: Design): Promise<string> => call(commands.saveDesign(path, design)),
  saveDesignAs: (design: Design, suggestedName: string): Promise<string | null> =>
    call(commands.saveDesignAs(design, suggestedName)),
  validateDesign: (design: Design): Promise<DesignIssue[]> => call(commands.validateDesign(design)),

  setWindowTitle: (title: string): Promise<null> => call(commands.setWindowTitle(title)),
  setDirty: (dirty: boolean): Promise<null> => call(commands.setDirty(dirty)),
  confirmClose: (): Promise<null> => call(commands.confirmClose()),
  /** Calls `cb` when the user tries to close the window with unsaved changes. */
  onCloseRequested: async (cb: () => void): Promise<() => void> => listen(CLOSE_REQUESTED, () => cb()),
};

export type Api = typeof api;
