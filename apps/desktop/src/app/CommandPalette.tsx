// Ctrl+K: go to a page, file actions, open a recent design, add a component to the
// canvas, switch theme or language. Found by the label as shown, with or without accents
// and punctuation (`paletteFilter`).
import { useEffect, useState } from "react";
import { useNavigate } from "@tanstack/react-router";
import { useTranslation } from "react-i18next";
import { BookOpen, Calculator, FilePlus2, Ruler, FileText, FolderOpen, Gauge, Languages, Monitor, Moon, Save, SaveAll, Sun } from "lucide-react";
import {
  CommandDialog,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
} from "@/components/ui/command";
import { NAV, SETTINGS_NAV } from "@/app/nav";
import { paletteFilter } from "@/app/paletteFilter";
import { UI_LANGS } from "@/i18n";
import { THEMES, usePrefs, type ThemePref } from "@/lib/prefs";
import { usePatterns, useRecentFiles, useRegistry } from "@/lib/queries";
import { requestAdd } from "@/solve/bridge";
import { useFileActions } from "@/solve/files";
import { Glyph } from "@/solve/icons";
import { kindName } from "@/solve/registry";

const THEME_ICON: Record<ThemePref, typeof Sun> = { system: Monitor, light: Sun, dark: Moon };

export function CommandPalette({ open, onOpenChange }: { open: boolean; onOpenChange: (open: boolean) => void }) {
  const { t, i18n } = useTranslation();
  const navigate = useNavigate();
  const { theme, lang, setTheme, setLang } = usePrefs();
  const files = useFileActions();
  const recents = useRecentFiles();
  const registry = useRegistry();
  const patterns = usePatterns();
  // Each opening starts empty, even when reopened during the closing animation.
  const [search, setSearch] = useState("");
  useEffect(() => {
    if (open) setSearch("");
  }, [open]);

  const run = (action: () => void) => {
    onOpenChange(false);
    action();
  };

  const add = (req: Parameters<typeof requestAdd>[0]) =>
    run(() => {
      void navigate({ to: "/sandbox" });
      requestAdd(req);
    });

  return (
    <CommandDialog
      open={open}
      onOpenChange={onOpenChange}
      title={t("palette.open")}
      description={t("palette.placeholder")}
      filter={paletteFilter}
    >
      <CommandInput placeholder={t("palette.placeholder")} value={search} onValueChange={setSearch} />
      <CommandList>
        <CommandEmpty>{t("palette.empty")}</CommandEmpty>
        <CommandGroup heading={t("palette.pages")}>
          {[...NAV, SETTINGS_NAV].map((item) => (
            <CommandItem key={item.to} value={`page ${t(item.labelKey)}`} onSelect={() => run(() => void navigate({ to: item.to }))}>
              <item.icon aria-hidden />
              {t(item.labelKey)}
            </CommandItem>
          ))}
          <CommandItem value={`page ${t("numbers.title")}`} onSelect={() => run(() => void navigate({ to: "/numbers" }))}>
            <Calculator aria-hidden />
            {t("numbers.title")}
          </CommandItem>
          <CommandItem value={`page ${t("estimate.title")}`} onSelect={() => run(() => void navigate({ to: "/estimate", search: {} }))}>
            <Ruler aria-hidden />
            {t("estimate.title")}
          </CommandItem>
          {import.meta.env.DEV && (
            <CommandItem value={`page ${t("palette.devPerf")}`} onSelect={() => run(() => void navigate({ to: "/dev/perf" }))}>
              <Gauge aria-hidden />
              {t("palette.devPerf")}
            </CommandItem>
          )}
        </CommandGroup>
        <CommandGroup heading={t("palette.file")}>
          <CommandItem value={`file ${t("files.new")}`} onSelect={() => run(() => void files.newDesign())}>
            <FilePlus2 aria-hidden />
            {t("files.new")}
          </CommandItem>
          <CommandItem value={`file ${t("files.open")}`} onSelect={() => run(() => void files.open())}>
            <FolderOpen aria-hidden />
            {t("files.open")}
          </CommandItem>
          <CommandItem value={`file ${t("files.save")}`} onSelect={() => run(() => void files.save())}>
            <Save aria-hidden />
            {t("files.save")}
          </CommandItem>
          <CommandItem value={`file ${t("files.saveAs")}`} onSelect={() => run(() => void files.saveAs())}>
            <SaveAll aria-hidden />
            {t("files.saveAs")}
          </CommandItem>
          {(recents.data ?? [])
            .filter((r) => r.exists)
            .map((r) => (
              <CommandItem
                key={r.path}
                // By name only: fuzzy matching on a long path matches almost any query.
                value={`recent ${t("palette.openRecent", { name: r.name })}`}
                onSelect={() => run(() => void files.openRecent(r.path))}
              >
                <FileText aria-hidden />
                {t("palette.openRecent", { name: r.name })}
              </CommandItem>
            ))}
        </CommandGroup>
        <CommandGroup heading={t("palette.guide")}>
          {(registry.data?.kinds ?? []).map((k) => (
            <CommandItem
              key={k.kind}
              value={`guide ${t("palette.guideItem", { name: kindName(t, k.kind) })}`}
              onSelect={() => run(() => void navigate({ to: "/guide/$kind", params: { kind: k.kind } }))}
            >
              <BookOpen aria-hidden />
              {t("palette.guideItem", { name: kindName(t, k.kind) })}
            </CommandItem>
          ))}
        </CommandGroup>
        <CommandGroup heading={t("glossary.title")}>
          <CommandItem value={`page ${t("glossary.title")}`} onSelect={() => run(() => void navigate({ to: "/glossary" }))}>
            <BookOpen aria-hidden />
            {t("glossary.title")}
          </CommandItem>
        </CommandGroup>
        <CommandGroup heading={t("palette.patterns")}>
          {(patterns.data ?? []).map((p) => {
            const title = i18n.language.startsWith("es") ? p.title_es : p.title;
            return (
              <CommandItem
                key={p.id}
                value={`pattern ${t("palette.patternItem", { name: title })}`}
                onSelect={() => run(() => void navigate({ to: "/patterns/$id", params: { id: p.id } }))}
              >
                <BookOpen aria-hidden />
                {t("palette.patternItem", { name: title })}
              </CommandItem>
            );
          })}
        </CommandGroup>
        <CommandGroup heading={t("palette.add")}>
          {(registry.data?.kinds ?? []).map((k) => (
            <CommandItem
              key={k.kind}
              value={`add ${t("palette.addItem", { name: kindName(t, k.kind) })}`}
              onSelect={() => add({ kind: k.kind })}
            >
              <span style={{ color: `var(--cat-${k.category}-glyph)` }}>
                <Glyph icon={k.icon} />
              </span>
              {t("palette.addItem", { name: kindName(t, k.kind) })}
            </CommandItem>
          ))}
          {(["region", "az", "note"] as const).map((m) => (
            <CommandItem
              key={m}
              value={`add ${t("palette.addItem", { name: t(`canvas.zone.${m}`) })}`}
              onSelect={() => add({ meta: m })}
            >
              <Glyph icon={m === "note" ? "note" : "zone"} />
              {t("palette.addItem", { name: t(`canvas.zone.${m}`) })}
            </CommandItem>
          ))}
        </CommandGroup>
        <CommandGroup heading={t("palette.theme")}>
          {THEMES.map((th) => {
            const Icon = THEME_ICON[th];
            return (
              <CommandItem
                key={th}
                value={`theme ${t("palette.theme")}: ${t(`theme.${th}`)}`}
                onSelect={() => run(() => setTheme(th))}
                data-checked={theme === th}
              >
                <Icon aria-hidden />
                {t("palette.theme")}: {t(`theme.${th}`)}
              </CommandItem>
            );
          })}
        </CommandGroup>
        <CommandGroup heading={t("palette.language")}>
          {UI_LANGS.map((l) => (
            <CommandItem
              key={l}
              value={`language ${t("palette.language")}: ${t(`language.${l}`)}`}
              onSelect={() => run(() => setLang(l))}
              data-checked={lang === l}
            >
              <Languages aria-hidden />
              {t("palette.language")}: {t(`language.${l}`)}
            </CommandItem>
          ))}
        </CommandGroup>
      </CommandList>
    </CommandDialog>
  );
}
