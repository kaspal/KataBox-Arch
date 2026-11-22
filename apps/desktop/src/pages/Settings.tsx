// Settings: appearance (theme, contrast, motion), language, canvas, about.
import { useTranslation } from "react-i18next";
import { Monitor, Moon, Sun } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Page } from "@/components/Page";
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group";
import { UI_LANGS, type UiLangCode } from "@/i18n";
import { CONTRASTS, MOTIONS, THEMES, usePrefs, type ContrastPref, type MotionPref, type ThemePref } from "@/lib/prefs";
import { useAppInfo } from "@/lib/queries";
import { useSnap } from "@/lib/settings";
import { PacksSettings } from "@/packs/PackImport";

const THEME_ICON: Record<ThemePref, typeof Sun> = { system: Monitor, light: Sun, dark: Moon };

function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <section className="space-y-4 rounded-xl border bg-card p-6">
      <h2 className="font-display text-2xl font-semibold">{title}</h2>
      {children}
    </section>
  );
}

function Row({ label, hint, children }: { label: string; hint?: string; children: React.ReactNode }) {
  return (
    <div className="flex flex-wrap items-center justify-between gap-3">
      <div>
        <div className="font-medium">{label}</div>
        {hint && <div className="text-sm text-muted-foreground">{hint}</div>}
      </div>
      {children}
    </div>
  );
}

export function Settings() {
  const { t } = useTranslation();
  const prefs = usePrefs();
  const info = useAppInfo();
  const [snap, setSnap] = useSnap();
  return (
    <Page title={t("nav.settings")}>
      <div className="grid gap-6">
        <Section title={t("settings.appearance")}>
          <Row label={t("settings.theme")}>
            <ToggleGroup
              type="single"
              variant="outline"
              value={prefs.theme}
              onValueChange={(v) => v && prefs.setTheme(v as ThemePref)}
              aria-label={t("settings.theme")}
            >
              {THEMES.map((th) => {
                const Icon = THEME_ICON[th];
                return (
                  <ToggleGroupItem key={th} value={th} className="gap-2 px-3">
                    <Icon aria-hidden className="size-4" />
                    {t(`theme.${th}`)}
                  </ToggleGroupItem>
                );
              })}
            </ToggleGroup>
          </Row>
          <Row label={t("settings.contrast")} hint={t("settings.contrastHint")}>
            <ToggleGroup
              type="single"
              variant="outline"
              value={prefs.contrast}
              onValueChange={(v) => v && prefs.setContrast(v as ContrastPref)}
              aria-label={t("settings.contrast")}
            >
              {CONTRASTS.map((c) => (
                <ToggleGroupItem key={c} value={c} className="px-3">
                  {t(`settings.contrastValue.${c}`)}
                </ToggleGroupItem>
              ))}
            </ToggleGroup>
          </Row>
          <Row label={t("settings.motion")} hint={t("settings.motionHint")}>
            <ToggleGroup
              type="single"
              variant="outline"
              value={prefs.motion}
              onValueChange={(v) => v && prefs.setMotion(v as MotionPref)}
              aria-label={t("settings.motion")}
            >
              {MOTIONS.map((m) => (
                <ToggleGroupItem key={m} value={m} className="px-3">
                  {t(`settings.motionValue.${m}`)}
                </ToggleGroupItem>
              ))}
            </ToggleGroup>
          </Row>
        </Section>
        <Section title={t("settings.language")}>
          <Row label={t("settings.uiLanguage")}>
            <ToggleGroup
              type="single"
              variant="outline"
              value={prefs.lang}
              onValueChange={(v) => v && prefs.setLang(v as UiLangCode)}
              aria-label={t("settings.uiLanguage")}
            >
              {UI_LANGS.map((l) => (
                <ToggleGroupItem key={l} value={l} className="px-3">
                  {t(`language.${l}`)}
                </ToggleGroupItem>
              ))}
            </ToggleGroup>
          </Row>
        </Section>
        <Section title={t("settings.canvas")}>
          <Row label={t("canvas.snap")} hint={t("canvas.snapHint")}>
            <ToggleGroup
              type="single"
              variant="outline"
              value={snap ? "on" : "off"}
              onValueChange={(v) => v && setSnap(v === "on")}
              aria-label={t("canvas.snap")}
            >
              <ToggleGroupItem value="on" className="px-3">
                {t("common.on")}
              </ToggleGroupItem>
              <ToggleGroupItem value="off" className="px-3">
                {t("common.off")}
              </ToggleGroupItem>
            </ToggleGroup>
          </Row>
        </Section>
        <PacksSettings />
        <Section title={t("settings.about")}>
          <Row label={t("settings.version")}>
            <span className="flex items-center gap-2 font-mono text-sm">
              {info.data?.version ?? "…"}
              {info.data?.dev_data && <Badge className="bg-dev-badge text-dev-badge-foreground">{t("app.devData")}</Badge>}
            </span>
          </Row>
          <p className="text-sm text-muted-foreground">{t("settings.privacy")}</p>
          <p className="text-sm text-muted-foreground">{t("settings.trademarks")}</p>
        </Section>
      </div>
    </Page>
  );
}
