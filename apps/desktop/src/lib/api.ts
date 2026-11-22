// The UI's only door to the backend. Wraps the generated tauri-specta commands: unwraps
// `{status, data | error}` and throws the typed AppError. Components use the hooks in
// `queries.ts` (or call `api` for actions); tests mock this module (`test/mockApi.ts`).
import { Channel } from "@tauri-apps/api/core";
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
  type RunControl,
  type RunInfo,
  type RunResult,
  type SandboxPreset,
} from "@/bindings";

export async function call<T>(p: Promise<Result<T, AppError>>): Promise<T> {
  const r = await p;
  if (r.status === "error") throw r.error;
  return r.data;
}

/** A frame arrives as an ArrayBuffer (raw IPC body); accept the other shapes defensively. */
function toArrayBuffer(data: unknown): ArrayBuffer {
  if (data instanceof ArrayBuffer) return data;
  if (ArrayBuffer.isView(data)) return data.buffer.slice(data.byteOffset, data.byteOffset + data.byteLength) as ArrayBuffer;
  if (Array.isArray(data)) return Uint8Array.from(data as number[]).buffer;
  throw new Error("unexpected frame payload");
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

  /** Start a live Sandbox run; `onFrame` receives each binary frame (`solve/run/frames.ts`). */
  startRun: (design: Design, preset: SandboxPreset, speed: number, onFrame: (frame: ArrayBuffer) => void): Promise<RunInfo> => {
    const channel = new Channel<unknown>();
    channel.onmessage = (data) => onFrame(toArrayBuffer(data));
    // The channel serializes to its id; the bindings type that field as a string.
    return call(commands.startRun({ design, preset, speed, on_frame: channel as unknown as string }));
  },
  controlRun: (runId: number, control: RunControl): Promise<null> => call(commands.controlRun(runId, control)),
  runResult: (runId: number): Promise<RunResult | null> => call(commands.runResult(runId)),

  setWindowTitle: (title: string): Promise<null> => call(commands.setWindowTitle(title)),
  setDirty: (dirty: boolean): Promise<null> => call(commands.setDirty(dirty)),
  confirmClose: (): Promise<null> => call(commands.confirmClose()),
  /** Calls `cb` when the user tries to close the window with unsaved changes. */
  onCloseRequested: async (cb: () => void): Promise<() => void> => listen(CLOSE_REQUESTED, () => cb()),
};

export type Api = typeof api;
