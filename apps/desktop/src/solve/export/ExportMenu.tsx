// The toolbar's Export menu: the design as a Mermaid diagram (M6.6), copied or saved. Rust
// writes the text (ka_core::mermaid) from names in the user's language; saving opens the
// dialog in Rust.
import { useState } from "react";
import { useTranslation } from "react-i18next";
import type { TFunction } from "i18next";
import { Check, Copy, FileDown, Share2 } from "lucide-react";
import type { Registry } from "@/bindings";
import { useReportError } from "@/app/dialogs";
import { Button } from "@/components/ui/button";
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuLabel, DropdownMenuTrigger } from "@/components/ui/dropdown-menu";
import { api } from "@/lib/api";
import { useRegistry } from "@/lib/queries";
import { kindName } from "@/solve/registry";
import { toDesign, useEditor } from "@/solve/store";

const PROTOCOLS = ["http", "grpc", "ws", "query", "publish", "consume", "replicate"] as const;
const GROUPS = ["region", "az", "autoscale"] as const;

/** Kind, protocol and group names for the export, in the user's language. */
export function exportNames(t: TFunction, registry: Registry): Record<string, string> {
  const names: Record<string, string> = {};
  for (const k of registry.kinds) names[`kind.${k.kind}`] = kindName(t, k.kind);
  for (const p of PROTOCOLS) names[`protocol.${p}`] = t(`protocol.${p}` as "protocol.http");
  for (const g of GROUPS) names[`group.${g}`] = t(`canvas.zone.${g}` as "canvas.zone.region");
  return names;
}

export function ExportMenu() {
  const { t } = useTranslation();
  const registry = useRegistry();
  const report = useReportError();
  const [copied, setCopied] = useState(false);
  const names = () => (registry.data ? exportNames(t, registry.data) : {});
  const design = () => toDesign(useEditor.getState());

  const copy = async () => {
    try {
      const text = await api.mermaidText(design(), names());
      await navigator.clipboard.writeText(text);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch (e) {
      report(e);
    }
  };
  const save = async () => {
    try {
      const d = design();
      await api.saveMermaid(d, names(), `${d.name || "design"}.mmd`);
    } catch (e) {
      report(e);
    }
  };

  return (
    <>
      <DropdownMenu>
        <DropdownMenuTrigger asChild>
          <Button variant="ghost" size="icon" className="size-8" aria-label={t("export.menu")} data-testid="export-menu">
            <Share2 aria-hidden className="size-4" />
          </Button>
        </DropdownMenuTrigger>
        <DropdownMenuContent align="end">
          <DropdownMenuLabel>{t("export.menu")}</DropdownMenuLabel>
          <DropdownMenuItem onSelect={() => void copy()} className="gap-2">
            {copied ? <Check aria-hidden className="size-4" /> : <Copy aria-hidden className="size-4" />}
            {t("export.copyMermaid")}
          </DropdownMenuItem>
          <DropdownMenuItem onSelect={() => void save()} className="gap-2">
            <FileDown aria-hidden className="size-4" />
            {t("export.saveMermaid")}
          </DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>
      <span className="sr-only" role="status" aria-live="polite">
        {copied ? t("export.copied") : ""}
      </span>
    </>
  );
}
