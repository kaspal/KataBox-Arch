// The app frame (matches the Home mockup): a paper sidebar with the brand and the
// navigation on the desk, the page on a sheet next to it, plus the global pieces: the
// Ctrl+K palette, file shortcuts, the window title, and the unsaved-changes close guard.
import { useEffect, useState } from "react";
import { Link, Outlet, useRouterState } from "@tanstack/react-router";
import { useTranslation } from "react-i18next";
import { Command as CommandIcon } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { CommandPalette } from "@/app/CommandPalette";
import { Notice, UnsavedDialog, askUnsaved } from "@/app/dialogs";
import { ComposeSummary } from "@/solve/ComposeSummary";
import { AnimationDialog } from "@/solve/export/AnimationDialog";
import { PrintFallbackDialog } from "@/print/PrintMenu";
import { NAV, SETTINGS_NAV, type NavItem } from "@/app/nav";
import { api } from "@/lib/api";
import { useAppInfo } from "@/lib/queries";
import { displayName, useFileActions } from "@/solve/files";
import { useDirty, useEditor } from "@/solve/store";

const APP_TITLE = "KataBox Arch";

/** Keep the window title and Rust's close guard in step with the open design. */
function useWindowSync() {
  const dirty = useDirty();
  const name = useEditor((s) => s.doc.name);
  const path = useEditor((s) => s.path);
  const files = useFileActions();

  useEffect(() => {
    void api.setDirty(dirty).catch(() => {});
  }, [dirty]);

  useEffect(() => {
    const title = path || dirty ? `${displayName(path, name)}${dirty ? " •" : ""} — ${APP_TITLE}` : APP_TITLE;
    document.title = title;
    void api.setWindowTitle(title).catch(() => {});
  }, [dirty, name, path]);

  // Rust holds a close back while the design is dirty and asks us (title-bar X, Alt+F4,
  // the taskbar's Close all arrive here).
  useEffect(() => {
    let off: (() => void) | undefined;
    let live = true;
    void api
      .onCloseRequested(async () => {
        const choice = await askUnsaved(useEditor.getState().doc.name);
        if (choice === "cancel") return;
        if (choice === "save" && !(await files.save())) return;
        await api.confirmClose().catch(() => {});
      })
      .then((f) => {
        if (live) off = f;
        else f();
      })
      .catch(() => {});
    return () => {
      live = false;
      off?.();
    };
  }, [files]);
}

function NavLink({ item }: { item: NavItem }) {
  const { t } = useTranslation();
  return (
    <Link
      to={item.to}
      activeOptions={{ exact: item.to === "/" }}
      className="flex items-center gap-3 rounded-lg border border-transparent px-3 py-2 text-[15px] text-muted-foreground transition-colors hover:bg-accent hover:text-foreground aria-[current=page]:border-border aria-[current=page]:bg-card aria-[current=page]:font-medium aria-[current=page]:text-foreground"
    >
      <item.icon aria-hidden className="size-[18px]" />
      {t(item.labelKey)}
    </Link>
  );
}

export function Shell() {
  const { t } = useTranslation();
  const [paletteOpen, setPaletteOpen] = useState(false);
  const printing = useRouterState({ select: (st) => st.location.pathname.startsWith("/print") });
  const info = useAppInfo();
  const files = useFileActions();
  useWindowSync();

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (!(e.ctrlKey || e.metaKey) || e.altKey) return;
      const k = e.key.toLowerCase();
      if (k === "k") {
        e.preventDefault();
        setPaletteOpen((open) => !open);
      } else if (k === "n" && !e.shiftKey) {
        e.preventDefault();
        void files.newDesign();
      } else if (k === "o" && !e.shiftKey) {
        e.preventDefault();
        void files.open();
      } else if (k === "s") {
        e.preventDefault();
        void (e.shiftKey ? files.saveAs() : files.save());
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [files]);

  const dialogs = (
    <>
      <CommandPalette open={paletteOpen} onOpenChange={setPaletteOpen} />
      <UnsavedDialog />
      <Notice />
      <ComposeSummary />
      <AnimationDialog />
      <PrintFallbackDialog />
    </>
  );
  // The print layout (M7b.3) is paper: no rail, no frame.
  if (printing) {
    return (
      <>
        <main id="main" tabIndex={-1} className="h-full overflow-auto bg-white outline-none">
          <Outlet />
        </main>
        {dialogs}
      </>
    );
  }

  return (
    <div className="flex h-full gap-0 bg-desk p-3">
      <a
        href="#main"
        data-testid="skip-link"
        className="sr-only z-50 rounded-md bg-primary text-primary-foreground focus:not-sr-only focus:fixed focus:left-2 focus:top-2 focus:px-3 focus:py-2 focus:text-sm focus:font-medium"
        onClick={(e) => {
          // The router uses the hash: move focus instead of navigating.
          e.preventDefault();
          document.getElementById("main")?.focus();
        }}
      >
        {t("app.skipToContent")}
      </a>
      <div className="flex min-w-0 flex-1 overflow-hidden rounded-2xl border bg-background shadow-[0_1px_0_var(--desk)]">
        <nav aria-label={t("nav.label")} className="flex w-56 shrink-0 flex-col border-r bg-background px-4 py-5">
          <Link to="/" className="mb-6 flex items-baseline gap-1.5 px-2 font-display" aria-label={APP_TITLE}>
            <span className="text-[28px] font-semibold leading-none text-primary-ink">KataBox</span>
            <span className="text-xl italic leading-none text-terracotta-ink">Arch</span>
          </Link>
          <button
            type="button"
            onClick={() => setPaletteOpen(true)}
            className="mb-4 flex items-center justify-between rounded-lg border bg-card px-3 py-1.5 text-sm text-muted-foreground hover:text-foreground"
          >
            <span className="flex items-center gap-2">
              <CommandIcon aria-hidden className="size-3.5" />
              {t("palette.open")}
            </span>
            <kbd className="rounded border bg-muted px-1.5 font-mono text-[11px]">Ctrl K</kbd>
          </button>
          <div className="flex flex-col gap-1">
            {NAV.map((item) => (
              <NavLink key={item.to} item={item} />
            ))}
          </div>
          <div className="mt-auto flex flex-col gap-3">
            {info.data?.dev_data && (
              <Tooltip>
                <TooltipTrigger asChild>
                  <Badge
                    tabIndex={0}
                    className="self-start bg-dev-badge font-mono text-dev-badge-foreground"
                    data-testid="dev-data-badge"
                  >
                    {t("app.devData")}
                  </Badge>
                </TooltipTrigger>
                <TooltipContent>{t("app.devDataHint")}</TooltipContent>
              </Tooltip>
            )}
            <NavLink item={SETTINGS_NAV} />
          </div>
        </nav>
        <main id="main" tabIndex={-1} className="min-h-0 min-w-0 flex-1 overflow-auto outline-none">
          <Outlet />
        </main>
      </div>
      {dialogs}
    </div>
  );
}
