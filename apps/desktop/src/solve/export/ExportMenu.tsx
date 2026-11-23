// The toolbar's Export menu: the design as a Mermaid diagram (M6.6), copied or saved. Rust
// writes the text (ka_core::mermaid) from names in the user's language; saving opens the
// dialog in Rust.
import { useState } from "react";
import { useTranslation } from "react-i18next";
import type { TFunction } from "i18next";
import { Check, Copy, FileDown, FileImage, Film, Link2, Share2 } from "lucide-react";
import type { Registry } from "@/bindings";
import { useReportError } from "@/app/dialogs";
import { Button } from "@/components/ui/button";
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuLabel, DropdownMenuTrigger } from "@/components/ui/dropdown-menu";
import { api } from "@/lib/api";
import { ShareTooLarge, shareLink } from "@/lib/share";
import { useRegistry } from "@/lib/queries";
import { flavourName, kindName } from "@/solve/registry";
import { designSvg, readPalette, svgToPng, type SceneNames } from "@/solve/export/scene";
import { toDesign, useEditor } from "@/solve/store";
import { useRun } from "@/solve/run/runStore";
import { useProblemMode } from "@/solve/problem/context";
import { openAnimation } from "@/solve/export/animationStore";
import { webmType } from "@/solve/export/animation";

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
  const { t, i18n } = useTranslation();
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
  const [linked, setLinked] = useState(false);
  const share = async () => {
    try {
      const link = await shareLink(await api.sharePayload(design()));
      await navigator.clipboard.writeText(link);
      setLinked(true);
      setTimeout(() => setLinked(false), 2000);
    } catch (e) {
      report(e instanceof ShareTooLarge ? new Error(t("export.shareTooLarge")) : e);
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

  // The run the Export menu animates: the problem test Run plays, else the Sandbox preset.
  const mode = useProblemMode();
  const preset = useRun((s) => s.preset);
  const problemTest = useRun((s) => s.problemTest);
  const webm = webmType();
  const animate = () => {
    const d = design();
    const test = problemTest ? mode?.brief.tests.find((x) => x.id === problemTest.test_id) : undefined;
    if (problemTest && test) {
      openAnimation({
        source: { type: "test", design: d, problem_id: problemTest.problem_id, test_id: test.id },
        label: i18n.language.startsWith("es") ? test.name_es : test.name,
        minStart: 0,
        durationS: test.duration_s,
      });
    } else {
      openAnimation({
        source: { type: "sandbox", design: d, preset },
        label: `${t("animation.sandbox")} · ${t(`run.presets.${preset}`)}`,
        minStart: 0,
        durationS: 60,
      });
    }
  };

  const image = async (format: "svg" | "png") => {
    if (!registry.data) return;
    try {
      const d = design();
      const sceneNames: SceneNames = {
        kind: (k) => kindName(t, k as never),
        flavour: (f) => flavourName(t, f as never),
        protocol: (p) => t(`protocol.${p}` as "protocol.http"),
        zone: (z) => t(`canvas.zone.${z}` as "canvas.zone.region"),
      };
      // Webfonts first (review A7), so text measures the same as on screen.
      await document.fonts?.ready;
      const svg = designSvg(d, registry.data, sceneNames, readPalette());
      const bytes = format === "svg" ? new TextEncoder().encode(svg) : await svgToPng(svg);
      await api.saveExport(format, bytes, `${d.name || "design"}.${format}`);
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
          <DropdownMenuItem onSelect={() => void share()} className="gap-2" data-testid="export-share-link">
            {linked ? <Check aria-hidden className="size-4" /> : <Link2 aria-hidden className="size-4" />}
            {t("export.copyShareLink")}
          </DropdownMenuItem>
          <DropdownMenuItem onSelect={() => void save()} className="gap-2">
            <FileDown aria-hidden className="size-4" />
            {t("export.saveMermaid")}
          </DropdownMenuItem>
          <DropdownMenuItem onSelect={() => void image("svg")} className="gap-2">
            <FileImage aria-hidden className="size-4" />
            {t("export.saveSvg")}
          </DropdownMenuItem>
          <DropdownMenuItem onSelect={() => void image("png")} className="gap-2">
            <FileImage aria-hidden className="size-4" />
            {t("export.savePng")}
          </DropdownMenuItem>
          <DropdownMenuItem onSelect={animate} disabled={!webm} className="items-start gap-2" data-testid="export-webm">
            <Film aria-hidden className="mt-0.5 size-4" />
            <span>
              {t("export.saveWebm")}
              {!webm && <span className="block text-xs text-muted-foreground">{t("export.webmUnsupported")}</span>}
            </span>
          </DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>
      <span className="sr-only" role="status" aria-live="polite">
        {copied ? t("export.copied") : ""}
      </span>
    </>
  );
}
