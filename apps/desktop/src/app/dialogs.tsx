// App-wide dialogs that non-React code can await: the unsaved-changes question, and a
// small notice for errors. Rendered once by the Shell.
import { useCallback } from "react";
import { create } from "zustand";
import { useTranslation } from "react-i18next";
import { X } from "lucide-react";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { describeError } from "@/lib/errors";

export type UnsavedChoice = "save" | "discard" | "cancel";

type DialogState = {
  unsaved: { name: string; resolve: (c: UnsavedChoice) => void } | null;
  notice: { summary: string; detail?: string } | null;
};

const useDialogs = create<DialogState>(() => ({ unsaved: null, notice: null }));

/** Ask Save / Discard / Cancel about unsaved changes to `name`. */
export function askUnsaved(name: string): Promise<UnsavedChoice> {
  return new Promise((resolve) => useDialogs.setState({ unsaved: { name, resolve } }));
}

/** Close every dialog (tests). Pending questions are answered Cancel. */
export function resetDialogs() {
  useDialogs.getState().unsaved?.resolve("cancel");
  useDialogs.setState({ unsaved: null, notice: null });
}

export function showNotice(summary: string, detail?: string) {
  useDialogs.setState({ notice: { summary, detail } });
}

export function UnsavedDialog() {
  const { t } = useTranslation();
  const unsaved = useDialogs((s) => s.unsaved);
  const answer = (c: UnsavedChoice) => {
    unsaved?.resolve(c);
    useDialogs.setState({ unsaved: null });
  };
  return (
    <Dialog open={unsaved !== null} onOpenChange={(open) => !open && answer("cancel")}>
      <DialogContent showCloseButton={false} data-testid="unsaved-dialog">
        <DialogHeader>
          <DialogTitle>{t("files.unsaved.title")}</DialogTitle>
          <DialogDescription>{t("files.unsaved.body", { name: unsaved?.name ?? "" })}</DialogDescription>
        </DialogHeader>
        <DialogFooter>
          <Button variant="ghost" onClick={() => answer("cancel")}>
            {t("common.cancel")}
          </Button>
          <Button variant="outline" onClick={() => answer("discard")}>
            {t("files.unsaved.discard")}
          </Button>
          <Button onClick={() => answer("save")} autoFocus>
            {t("files.unsaved.save")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

export function Notice() {
  const { t } = useTranslation();
  const notice = useDialogs((s) => s.notice);
  if (!notice) return null;
  return (
    <div
      role="alert"
      className="fixed bottom-4 right-4 z-50 flex max-w-md items-start gap-3 rounded-lg border bg-card p-3 text-sm shadow-lg"
      data-testid="notice"
    >
      <div className="min-w-0 flex-1">
        <p className="font-medium text-destructive">{notice.summary}</p>
        {notice.detail && <p className="mt-1 break-words font-mono text-xs text-muted-foreground">{notice.detail}</p>}
      </div>
      <Button
        variant="ghost"
        size="icon"
        className="size-7"
        aria-label={t("common.dismiss")}
        onClick={() => useDialogs.setState({ notice: null })}
      >
        <X aria-hidden className="size-4" />
      </Button>
    </div>
  );
}

/** Show an error from a command as a notice, translated by its code. */
/** Show an error as a notice. Stable across renders, so effects can depend on it. */
export function useReportError() {
  const { i18n } = useTranslation();
  return useCallback(
    (e: unknown) => {
      const d = describeError(i18n, e);
      showNotice(d.summary, d.detail);
    },
    [i18n],
  );
}
