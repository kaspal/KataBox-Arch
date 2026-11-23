// "Print" on the Guide, a Guide entry, a pattern and a learning path (M7b.3): Save as PDF
// (Rust prints the print page) or Save print-ready HTML (works everywhere). And the dialog
// that offers the HTML when a PDF couldn't be made.
import { useRouter, useRouterState } from "@tanstack/react-router";
import { useTranslation } from "react-i18next";
import { FileCode2, FileDown, Printer } from "lucide-react";
import { useReportError } from "@/app/dialogs";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger } from "@/components/ui/dropdown-menu";
import { api } from "@/lib/api";
import { describeError } from "@/lib/errors";
import { printFileName, resetPrintFallback, usePrintFallback, type PrintWhat } from "@/print/printing";

function usePrintActions() {
  const router = useRouter();
  const here = useRouterState({ select: (s) => s.location.href });
  const report = useReportError();
  const go = (what: PrintWhat, name: string, extra: { pdf?: number; html?: 1 }) =>
    void router.navigate({ to: "/print", search: { what, name, back: here, ...extra } });
  return {
    pdf: async (what: PrintWhat, name: string) => {
      try {
        const token = await api.pickPdfPath(printFileName(name, "pdf"));
        if (token !== null) go(what, name, { pdf: token });
      } catch (e) {
        report(e);
      }
    },
    html: (what: PrintWhat, name: string) => go(what, name, { html: 1 }),
  };
}

export function PrintMenu({ what, name, label }: { what: PrintWhat; name: string; label?: string }) {
  const { t } = useTranslation();
  const actions = usePrintActions();
  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button variant="outline" size="sm" className="gap-1.5" data-testid="print-menu">
          <Printer aria-hidden className="size-4" />
          {label ?? t("print.menu")}
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end">
        <DropdownMenuItem onSelect={() => void actions.pdf(what, name)} className="gap-2">
          <FileDown aria-hidden className="size-4" />
          {t("print.savePdf")}
        </DropdownMenuItem>
        <DropdownMenuItem onSelect={() => actions.html(what, name)} className="gap-2">
          <FileCode2 aria-hidden className="size-4" />
          {t("print.saveHtml")}
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

/** After a PDF failed (and the app went back): say why, and offer the HTML instead. */
export function PrintFallbackDialog() {
  const { t, i18n } = useTranslation();
  const { what, name, error } = usePrintFallback();
  const actions = usePrintActions();
  return (
    <Dialog open={what !== null} onOpenChange={(open) => !open && resetPrintFallback()}>
      <DialogContent className="sm:max-w-md" data-testid="print-fallback">
        <DialogHeader>
          <DialogTitle>{t("print.pdfFailedTitle")}</DialogTitle>
          <DialogDescription>{error ? describeError(i18n, error).summary : ""}</DialogDescription>
        </DialogHeader>
        <p className="text-sm">{t("print.pdfFailedBody")}</p>
        <DialogFooter>
          <Button variant="ghost" onClick={resetPrintFallback}>
            {t("common.close")}
          </Button>
          <Button
            onClick={() => {
              if (what) actions.html(what, name);
              resetPrintFallback();
            }}
            autoFocus
          >
            {t("print.saveHtml")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
