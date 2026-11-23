// The UI's only door to the backend. Wraps the generated tauri-specta commands: unwraps
// `{status, data | error}` and throws the typed AppError. Components use the hooks in
// `queries.ts` (or call `api` for actions); tests mock this module (`test/mockApi.ts`).
import { Channel, invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  commands,
  type AppError,
  type AppInfo,
  type BreakingPoint,
  type ChaosEvent,
  type ProblemBrief,
  type ProblemSummary,
  type ComponentKind,
  type GuidePage,
  type GuideSummary,
  type DecideFlow,
  type GlossaryTerm,
  type NumberFigure,
  type Drill,
  type EstimateCheck,
  type EstimateStatus,
  type FlawSummary,
  type OutageStory,
  type OutageSummary,
  type PathView,
  type QuizItem,
  type QuizResult,
  type PatternPage,
  type PatternSummary,
  type Sample,
  type SampleSummary,
  type ProblemTest,
  type Progress,
  type SubmissionSummary,
  type SubmitResult,
  type Design,
  type DesignIssue,
  type JsonValue,
  type ComposeImport,
  type OpenedDesign,
  type RecentEntry,
  type Registry,
  type Result,
  type RunControl,
  type RunInfo,
  type RunResult,
  type SandboxPreset,
  Judgement,
  KataboxLink,
  PackInfo,
  PackPreview,
} from "@/bindings";

/** What an export saves (`ka_app::ExportFormat`). */
export type ExportFormat = "svg" | "png" | "webm" | "gif" | "html";

/** Which run an animated export shows (`ka_app::frames::FramesSource`). */
export type FramesSource =
  | { type: "sandbox"; design: Design; preset: SandboxPreset }
  | { type: "test"; design: Design; problem_id: string; test_id: string }
  | { type: "submission"; submission_id: number; test_id: string };

export type FramesRequest = { source: FramesSource; from_s: number; seconds: number; fps: 15 | 30 };

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

/** How to run a design live: a Sandbox preset or a problem's test. */
export type RunOptions = {
  preset: SandboxPreset;
  problem: ProblemTest | null;
  speed: number;
  chaos: ChaosEvent[];
  startS: number;
};

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
  importComposeDialog: (): Promise<ComposeImport | null> => call(commands.importComposeDialog()),
  // Printable Guide (M7b.3): Rust's Save dialog keeps the path behind a one-shot token; the
  // print page then asks Rust to print itself to it. null when cancelled.
  pickPdfPath: (suggestedName: string): Promise<number | null> => call(commands.pickPdfPath(suggestedName)),
  printPdf: (token: number): Promise<string> => call(commands.printPdf(token)),
  saveDesign: (path: string, design: Design): Promise<string> => call(commands.saveDesign(path, design)),
  saveDesignAs: (design: Design, suggestedName: string): Promise<string | null> =>
    call(commands.saveDesignAs(design, suggestedName)),
  validateDesign: (design: Design): Promise<DesignIssue[]> => call(commands.validateDesign(design)),

  /** Start a live Sandbox run; `onFrame` receives each binary frame (`solve/run/frames.ts`). */
  startRun: (design: Design, opts: RunOptions, onFrame: (frame: ArrayBuffer) => void): Promise<RunInfo> => {
    const channel = new Channel<unknown>();
    channel.onmessage = (data) => onFrame(toArrayBuffer(data));
    // The channel serializes to its id; the bindings type that field as a string.
    return call(
      commands.startRun({
        design,
        preset: opts.preset,
        problem: opts.problem,
        speed: opts.speed,
        chaos: opts.chaos,
        start_s: opts.startS,
        on_frame: channel as unknown as string,
      }),
    );
  },
  controlRun: (runId: number, control: RunControl): Promise<null> => call(commands.controlRun(runId, control)),
  runResult: (runId: number): Promise<RunResult | null> => call(commands.runResult(runId)),
  /** How much traffic the design holds before it fails (a few seconds of simulation). */
  findBreakingPoint: (design: Design): Promise<BreakingPoint> => call(commands.findBreakingPoint(design)),

  listProblems: (): Promise<ProblemSummary[]> => call(commands.listProblems()),
  /** A problem's brief: hidden tests by name only, hints up to the unlocked tier. */
  problemBrief: (id: string): Promise<ProblemBrief> => call(commands.problemBrief(id)),
  unlockHint: (id: string, tier: number): Promise<null> => call(commands.unlockHint(id, tier)),
  saveDraft: (id: string, design: Design): Promise<null> => call(commands.saveDraft(id, design)),
  /** Judge on every test and record the submission (`localDay`: the user's YYYY-MM-DD). */
  submitDesign: (id: string, design: Design, localDay: string): Promise<SubmitResult> =>
    call(commands.submitDesign(id, design, localDay)),
  /** M8.5: a "same SLO, half the cost" try; opens once the problem is solved. */
  submitChallenge: (id: string, design: Design): Promise<Judgement> => call(commands.submitChallenge(id, design)),
  submissions: (id: string): Promise<SubmissionSummary[]> => call(commands.submissions(id)),
  progress: (today: string): Promise<Progress> => call(commands.progress(today)),
  /** The Component Guide index: one-liners and read marks, in registry order. */
  listGuide: (): Promise<GuideSummary[]> => call(commands.listGuide()),
  guidePage: (kind: ComponentKind): Promise<GuidePage> => call(commands.guidePage(kind)),
  setGuideRead: (kind: ComponentKind, read: boolean): Promise<null> => call(commands.setGuideRead(kind, read)),
  /** The "Which one do I need?" decision guides, in order. */
  decideFlows: (): Promise<DecideFlow[]> => call(commands.decideFlows()),
  glossary: (): Promise<GlossaryTerm[]> => call(commands.glossary()),
  /** The numbers cheat sheet, in sheet order. */
  numbers: (): Promise<NumberFigure[]> => call(commands.numbers()),
  /** Estimation drills (every problem's, or one problem's), without their answers. */
  drills: (problemId?: string): Promise<Drill[]> => call(commands.drills(problemId ?? null)),
  /** Judge an estimate (recorded) and reveal the answer with its worked solution. */
  checkEstimate: (problemId: string, drillId: string, answer: number): Promise<EstimateCheck> =>
    call(commands.checkEstimate(problemId, drillId, answer)),
  estimateStatuses: (): Promise<EstimateStatus[]> => call(commands.estimateStatuses()),
  /** The Find-the-flaw drills (each opens as the problem `flaw--<id>`). */
  listFlaws: (): Promise<FlawSummary[]> => call(commands.listFlaws()),
  /** The outage replays (M10), each marked fixed or not. */
  listOutages: (): Promise<OutageSummary[]> => call(commands.listOutages()),
  /** A replay's story, lesson and walkthrough. */
  outageStory: (id: string): Promise<OutageStory> => call(commands.outageStory(id)),
  /** The learning paths, each item with its progress. */
  learningPaths: (): Promise<PathView[]> => call(commands.learningPaths()),
  /** A page's quiz (`guide:<kind>` / `pattern:<id>`), without answers. */
  quiz: (page: string): Promise<QuizItem[]> => call(commands.quiz(page)),
  /** Answer a question (the option's index); `today` is the local date, YYYY-MM-DD. */
  answerQuiz: (questionId: string, choice: number, today: string): Promise<QuizResult> => call(commands.answerQuiz(questionId, choice, today)),
  /** Today's review questions (at most 10), in review order. */
  reviewsDue: (today: string): Promise<QuizItem[]> => call(commands.reviewsDue(today)),
  // KataBox practice related to an Arch page (M6.7): `guide:<kind>`, `pattern:<id>`, `problem:<id>`.
  kataboxLinks: (target: string): Promise<KataboxLink[]> => call(commands.kataboxLinks(target)),
  // Mermaid export (M6.6): names are kind, protocol and group names in the user's language.
  mermaidText: (design: Design, names: Record<string, string>): Promise<string> => commands.mermaidText(design, names),
  /** M13: the design as canonical .kbarch JSON for a share link (checked like Save). */
  sharePayload: (design: Design): Promise<string> => call(commands.sharePayload(design)),
  saveMermaid: (design: Design, names: Record<string, string>, suggestedName: string): Promise<string | null> =>
    call(commands.saveMermaid(design, names, suggestedName)),
  // Export (M7.3, M7b.2/3): the bytes the UI rendered go as a raw body, the format and name in
  // headers; Rust opens the Save dialog. null when cancelled. Not a specta command (raw body).
  saveExport: (format: ExportFormat, data: Uint8Array, suggestedName: string): Promise<string | null> =>
    invoke<string | null>("save_export", data, {
      headers: { "ka-format": format, "ka-name": encodeURIComponent(suggestedName) },
    }),
  // Animated export (M7b.2): a run's frames, stepped headless in Rust, as raw bytes
  // (`unpackFrames`). Not a specta command (raw response).
  runFrames: async (request: FramesRequest): Promise<ArrayBuffer> =>
    toArrayBuffer(await invoke<unknown>("run_frames", { request })),
  // Problem packs (M6.4): Rust opens the file dialog; null when cancelled.
  previewPack: (): Promise<PackPreview | null> => call(commands.previewPack()),
  applyPack: (token: number): Promise<PackInfo> => call(commands.applyPack(token)),
  cancelPack: (token: number): Promise<null> => call(commands.cancelPack(token)),
  listPacks: (): Promise<PackInfo[]> => call(commands.listPacks()),
  uninstallPack: (id: string): Promise<null> => call(commands.uninstallPack(id)),
  /** A concept card was played to its end. */
  setConceptPlayed: (kind: ComponentKind): Promise<void> => call(commands.setConceptPlayed(kind)).then(() => undefined),
  /** The pattern library, in order, each with its template. */
  listPatterns: (): Promise<PatternSummary[]> => call(commands.listPatterns()),
  pattern: (id: string): Promise<PatternPage> => call(commands.pattern(id)),
  setPatternRead: (id: string, read: boolean): Promise<null> => call(commands.setPatternRead(id, read)),
  /** A kind's concept card (null until it has one). */
  concept: (kind: ComponentKind): Promise<Sample | null> => call(commands.concept(kind)),
  /** The sample gallery, in order, each with its first step's design. */
  listSamples: (): Promise<SampleSummary[]> => call(commands.listSamples()),
  /** One sample for the player: steps and design snapshots. */
  sample: (id: string): Promise<Sample> => call(commands.sample(id)),

  setWindowTitle: (title: string): Promise<null> => call(commands.setWindowTitle(title)),
  setDirty: (dirty: boolean): Promise<null> => call(commands.setDirty(dirty)),
  confirmClose: (): Promise<null> => call(commands.confirmClose()),
  /** Calls `cb` when the user tries to close the window with unsaved changes. */
  onCloseRequested: async (cb: () => void): Promise<() => void> => listen(CLOSE_REQUESTED, () => cb()),
};

export type Api = typeof api;
