// New / Open / Save / Save As / Open recent, with the unsaved-changes guard. Files are
// read and written by Rust (dialogs included); this only moves designs between the
// editor store and `api`.
import { useCallback } from "react";
import { useNavigate } from "@tanstack/react-router";
import { useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import type { OpenedDesign } from "@/bindings";
import { askUnsaved, useReportError } from "@/app/dialogs";
import { api } from "@/lib/api";
import { queryKeys } from "@/lib/queries";
import { showComposeSummary } from "@/solve/ComposeSummary";
import { isDirty, toDesign, useEditor } from "@/solve/store";

/** A file name for Save As: the design's name without characters Windows refuses. */
export function suggestedFileName(name: string): string {
  const clean = name.replace(/[<>:"/\\|?*\u0000-\u001f]/g, "").trim() || "design";
  return `${clean.slice(0, 80)}.kbarch`;
}

/** The file name shown in the title bar: the path's last part, or the design's name. */
export function displayName(path: string | null, name: string): string {
  if (!path) return name;
  return path.split(/[\\/]/).pop() || name;
}

export function useFileActions() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const report = useReportError();

  const refreshRecents = useCallback(
    () => void queryClient.invalidateQueries({ queryKey: queryKeys.recentFiles }),
    [queryClient],
  );

  const saveAs = useCallback(async (): Promise<boolean> => {
    const s = useEditor.getState();
    try {
      const path = await api.saveDesignAs(toDesign(s), suggestedFileName(s.doc.name));
      if (path === null) return false;
      useEditor.getState().markSaved(path);
      refreshRecents();
      return true;
    } catch (e) {
      report(e);
      return false;
    }
  }, [refreshRecents, report]);

  const save = useCallback(async (): Promise<boolean> => {
    const s = useEditor.getState();
    if (!s.path) return saveAs();
    try {
      const path = await api.saveDesign(s.path, toDesign(s));
      useEditor.getState().markSaved(path);
      refreshRecents();
      return true;
    } catch (e) {
      report(e);
      return false;
    }
  }, [refreshRecents, report, saveAs]);

  /** True when the current design may be replaced (saved, discarded, or not dirty). */
  const confirmReplace = useCallback(async (): Promise<boolean> => {
    if (!isDirty()) return true;
    const choice = await askUnsaved(useEditor.getState().doc.name);
    if (choice === "cancel") return false;
    if (choice === "discard") return true;
    return save();
  }, [save]);

  const show = useCallback(
    (opened: OpenedDesign) => {
      useEditor.getState().load(opened.design, opened.path);
      refreshRecents();
      // A design made for a problem opens on that problem's page.
      const problem = opened.design.problem_id;
      if (problem) void navigate({ to: "/problems/$id", params: { id: problem } });
      else void navigate({ to: "/sandbox" });
    },
    [navigate, refreshRecents],
  );

  const newDesign = useCallback(async () => {
    if (!(await confirmReplace())) return;
    useEditor.getState().newDesign(t("files.untitled"));
    void navigate({ to: "/sandbox" });
  }, [confirmReplace, navigate, t]);

  const open = useCallback(async () => {
    if (!(await confirmReplace())) return;
    try {
      const opened = await api.openDesignDialog();
      if (opened) show(opened);
    } catch (e) {
      report(e);
    }
  }, [confirmReplace, report, show]);

  /** Import a docker-compose file as a new, unsaved design (M7b.1). */
  const importCompose = useCallback(async () => {
    if (!(await confirmReplace())) return;
    try {
      const result = await api.importComposeDialog();
      if (!result) return;
      useEditor.getState().loadUnsaved(result.design);
      void navigate({ to: "/sandbox" });
      showComposeSummary(result);
    } catch (e) {
      report(e);
    }
  }, [confirmReplace, navigate, report]);

  const openRecent = useCallback(
    async (path: string) => {
      if (useEditor.getState().path === path) {
        void navigate({ to: "/sandbox" });
        return;
      }
      if (!(await confirmReplace())) return;
      try {
        show(await api.openRecent(path));
      } catch (e) {
        report(e);
        refreshRecents();
      }
    },
    [confirmReplace, navigate, refreshRecents, report, show],
  );

  const forgetRecent = useCallback(
    async (path: string) => {
      try {
        await api.forgetRecent(path);
      } catch (e) {
        report(e);
      }
      refreshRecents();
    },
    [refreshRecents, report],
  );

  return { newDesign, open, importCompose, openRecent, forgetRecent, save, saveAs, confirmReplace };
}
