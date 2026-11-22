// The Sandbox (matches the Solve mockup, without a brief): library | canvas | inspector,
// with the file and edit actions on top. Run / Submit / AI review arrive in M1 / M2 / M4.
import { useCallback, useEffect, useRef } from "react";
import { useTranslation } from "react-i18next";
import {
  FilePlus2,
  FolderOpen,
  Grid3x3,
  Play,
  Redo2,
  Save,
  SaveAll,
  Send,
  Sparkles,
  Undo2,
} from "lucide-react";
import { useStore } from "zustand";
import type { Registry } from "@/bindings";
import { Button } from "@/components/ui/button";
import { ResizableHandle, ResizablePanel, ResizablePanelGroup } from "@/components/ui/resizable";
import { Skeleton } from "@/components/ui/skeleton";
import { Toggle } from "@/components/ui/toggle";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { useRegistry } from "@/lib/queries";
import { parseLayout, useSetting, useSnap, type SandboxLayout } from "@/lib/settings";
import { Canvas } from "@/solve/Canvas";
import { displayName, useFileActions } from "@/solve/files";
import { InspectorPanel } from "@/solve/InspectorPanel";
import { LibraryPanel } from "@/solve/LibraryPanel";
import { redo, undo, useDirty, useEditor } from "@/solve/store";

function IconAction({
  label,
  shortcut,
  onClick,
  disabled,
  children,
}: {
  label: string;
  shortcut?: string;
  onClick?: () => void;
  disabled?: boolean;
  children: React.ReactNode;
}) {
  return (
    <Tooltip>
      <TooltipTrigger asChild>
        {/* A disabled button gets no pointer events: the span keeps its tooltip working. */}
        <span tabIndex={disabled ? 0 : -1} className="inline-flex outline-none">
          <Button variant="ghost" size="icon" className="size-8" aria-label={label} onClick={onClick} disabled={disabled}>
            {children}
          </Button>
        </span>
      </TooltipTrigger>
      <TooltipContent>{shortcut ? `${label} (${shortcut})` : label}</TooltipContent>
    </Tooltip>
  );
}

function Later({ label, tip, icon: Icon, primary }: { label: string; tip: string; icon: typeof Play; primary?: boolean }) {
  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <span tabIndex={0} className="inline-flex rounded-md" aria-label={`${label}: ${tip}`}>
          <Button variant={primary ? "default" : "outline"} size="sm" className="gap-2" disabled tabIndex={-1}>
            <Icon aria-hidden className="size-4" />
            {label}
          </Button>
        </span>
      </TooltipTrigger>
      <TooltipContent>{tip}</TooltipContent>
    </Tooltip>
  );
}

function Toolbar() {
  const { t } = useTranslation();
  const files = useFileActions();
  const name = useEditor((s) => s.doc.name);
  const path = useEditor((s) => s.path);
  const dirty = useDirty();
  const canUndo = useStore(useEditor.temporal, (s) => s.pastStates.length > 0);
  const canRedo = useStore(useEditor.temporal, (s) => s.futureStates.length > 0);
  const [snap, setSnap] = useSnap();
  return (
    <header className="flex h-14 shrink-0 items-center justify-between gap-4 border-b bg-background px-4">
      <div className="flex min-w-0 items-center gap-3">
        <h1 className="truncate font-display text-2xl font-semibold leading-none" data-testid="design-name">
          {name}
        </h1>
        <span className="truncate font-mono text-xs text-muted-foreground" data-testid="file-name">
          {path ? displayName(path, name) : t("files.notSaved")}
          {dirty && (
            <span className="ml-1 text-terracotta-ink" aria-label={t("files.unsavedChanges")}>
              ●
            </span>
          )}
        </span>
      </div>
      <div className="flex items-center gap-1" role="toolbar" aria-label={t("sandbox.toolbar")}>
        <IconAction label={t("files.new")} shortcut="Ctrl+N" onClick={() => void files.newDesign()}>
          <FilePlus2 aria-hidden className="size-4" />
        </IconAction>
        <IconAction label={t("files.open")} shortcut="Ctrl+O" onClick={() => void files.open()}>
          <FolderOpen aria-hidden className="size-4" />
        </IconAction>
        <IconAction label={t("files.save")} shortcut="Ctrl+S" onClick={() => void files.save()}>
          <Save aria-hidden className="size-4" />
        </IconAction>
        <IconAction label={t("files.saveAs")} shortcut="Ctrl+Shift+S" onClick={() => void files.saveAs()}>
          <SaveAll aria-hidden className="size-4" />
        </IconAction>
        <span className="mx-1 h-5 w-px bg-border" aria-hidden />
        <IconAction label={t("edit.undo")} shortcut="Ctrl+Z" onClick={undo} disabled={!canUndo}>
          <Undo2 aria-hidden className="size-4" />
        </IconAction>
        <IconAction label={t("edit.redo")} shortcut="Ctrl+Y" onClick={redo} disabled={!canRedo}>
          <Redo2 aria-hidden className="size-4" />
        </IconAction>
        <Tooltip>
          <TooltipTrigger asChild>
            <Toggle size="sm" pressed={snap} onPressedChange={(v) => setSnap(v)} aria-label={t("canvas.snap")} className="gap-1.5">
              <Grid3x3 aria-hidden className="size-4" />
            </Toggle>
          </TooltipTrigger>
          <TooltipContent>{t("canvas.snapHint")}</TooltipContent>
        </Tooltip>
      </div>
      <div className="flex items-center gap-2">
        <Later label={t("sandbox.run")} tip={t("sandbox.later", { milestone: "M1" })} icon={Play} />
        <Later label={t("sandbox.aiReview")} tip={t("sandbox.later", { milestone: "M4" })} icon={Sparkles} />
        <Later label={t("sandbox.submit")} tip={t("sandbox.later", { milestone: "M2" })} icon={Send} primary />
      </div>
    </header>
  );
}

const isTyping = (t: EventTarget | null) =>
  t instanceof HTMLElement && (t.isContentEditable || ["INPUT", "TEXTAREA", "SELECT"].includes(t.tagName));

function Workspace({ registry }: { registry: Registry }) {
  const { t } = useTranslation();
  const [layout, setLayout] = useSetting<SandboxLayout>("sandbox.layout", {}, parseLayout, { mirror: true });
  const timer = useRef<ReturnType<typeof setTimeout>>(undefined);
  const saveLayout = useCallback(
    (l: SandboxLayout) => {
      clearTimeout(timer.current);
      timer.current = setTimeout(() => setLayout(l), 400);
    },
    [setLayout],
  );

  // Undo / redo anywhere in the Sandbox except while typing.
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (!(e.ctrlKey || e.metaKey) || isTyping(e.target)) return;
      const k = e.key.toLowerCase();
      if (k === "z" && !e.shiftKey) {
        e.preventDefault();
        undo();
      } else if (k === "y" || (k === "z" && e.shiftKey)) {
        e.preventDefault();
        redo();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  return (
    <ResizablePanelGroup orientation="horizontal" className="min-h-0 flex-1" defaultLayout={layout} onLayoutChanged={saveLayout}>
      <ResizablePanel id="library" defaultSize="20" minSize="14" maxSize="35">
        <LibraryPanel registry={registry} />
      </ResizablePanel>
      <ResizableHandle />
      <ResizablePanel id="canvas" defaultSize="58" minSize="30">
        <section aria-label={t("canvas.label")} className="h-full">
          <Canvas registry={registry} />
        </section>
      </ResizablePanel>
      <ResizableHandle />
      <ResizablePanel id="inspector" defaultSize="22" minSize="16" maxSize="40">
        <InspectorPanel registry={registry} />
      </ResizablePanel>
    </ResizablePanelGroup>
  );
}

export function SandboxPage() {
  const registry = useRegistry();
  return (
    <div className="flex h-full min-h-0 flex-col" data-testid="sandbox">
      <Toolbar />
      {registry.data ? (
        <Workspace registry={registry.data} />
      ) : (
        <div className="flex flex-1 gap-4 p-4">
          <Skeleton className="h-full w-64" />
          <Skeleton className="h-full flex-1" />
        </div>
      )}
    </div>
  );
}
