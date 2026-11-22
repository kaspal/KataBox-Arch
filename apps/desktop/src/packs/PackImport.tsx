// Problem packs (M6.4, ADR 0011): "Import pack…" (Rust opens the file dialog, opens the
// pack and runs every check, judging included), a preview of what would be installed, and
// the Settings list of installed packs. Nothing is installed until the user confirms.
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { AlertTriangle, Loader2, Package, ShieldAlert, Trash2 } from "lucide-react";
import type { PackInfo, PackPreview } from "@/bindings";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { api } from "@/lib/api";
import { describeError } from "@/lib/errors";
import { queryKeys } from "@/lib/queries";

export const packsKey = ["packs"] as const;
export const PACKS_RECOMMEND = "packs.recommend";

export function usePacks() {
  return useQuery({ queryKey: packsKey, queryFn: api.listPacks });
}

/** Refresh everything a pack changes: the catalogue, progress, the packs list. */
function useRefresh() {
  const qc = useQueryClient();
  return () => {
    void qc.invalidateQueries({ queryKey: queryKeys.problems });
    void qc.invalidateQueries({ queryKey: queryKeys.progress });
    void qc.invalidateQueries({ queryKey: packsKey });
  };
}

function PreviewBody({ p }: { p: PackPreview }) {
  const { t, i18n } = useTranslation();
  const es = i18n.language.startsWith("es");
  const change =
    p.change.kind === "upgrade" ? t("packs.change.upgrade", { from: p.change.from }) : t(`packs.change.${p.change.kind}`);
  return (
    <div className="space-y-4 text-sm" data-testid="pack-preview">
      <p className="flex items-start gap-2 rounded-md border border-amber/40 bg-amber/10 p-3" data-testid="pack-unsigned">
        <ShieldAlert aria-hidden className="mt-0.5 size-4 shrink-0 text-amber-ink" />
        <span>{t("packs.unsigned", { author: p.author })}</span>
      </p>
      <dl className="grid grid-cols-[auto_1fr] gap-x-4 gap-y-1">
        <dt className="text-muted-foreground">{t("packs.field.name")}</dt>
        <dd className="font-semibold">{p.name}</dd>
        <dt className="text-muted-foreground">{t("packs.field.author")}</dt>
        <dd>{p.author}</dd>
        <dt className="text-muted-foreground">{t("packs.field.version")}</dt>
        <dd className="font-mono">{p.version}</dd>
        <dt className="text-muted-foreground">{t("packs.field.licence")}</dt>
        <dd className="font-mono">{p.licence}</dd>
        <dt className="text-muted-foreground">{t("packs.field.sha")}</dt>
        <dd className="break-all font-mono text-xs" data-testid="pack-sha">
          {p.sha256}
        </dd>
      </dl>
      <div>
        <p className="mb-1 font-semibold">{t("packs.problems", { count: p.problems.length })}</p>
        <ul className="list-inside list-disc text-muted-foreground">
          {p.problems.map((x) => (
            <li key={x.id}>{es ? x.title_es : x.title_en}</li>
          ))}
        </ul>
      </div>
      <p className="font-medium" data-testid="pack-change">
        {change}
      </p>
    </div>
  );
}

/** "Import pack…": the dialog flow. */
export function ImportPackButton() {
  const { t, i18n } = useTranslation();
  const refresh = useRefresh();
  const [busy, setBusy] = useState(false);
  const [preview, setPreview] = useState<PackPreview | null>(null);
  const [error, setError] = useState<{ summary: string; detail?: string } | null>(null);
  const [done, setDone] = useState<PackInfo | null>(null);

  const start = async () => {
    setBusy(true);
    setError(null);
    setDone(null);
    try {
      const p = await api.previewPack();
      if (p) setPreview(p);
    } catch (e) {
      setError(describeError(i18n, e));
    } finally {
      setBusy(false);
    }
  };
  const cancel = () => {
    if (preview) void api.cancelPack(preview.token).catch(() => {});
    setPreview(null);
  };
  const apply = async () => {
    if (!preview) return;
    setBusy(true);
    try {
      setDone(await api.applyPack(preview.token));
      setPreview(null);
      refresh();
    } catch (e) {
      setError(describeError(i18n, e));
      setPreview(null);
    } finally {
      setBusy(false);
    }
  };

  return (
    <>
      <Button variant="outline" className="gap-2 bg-card" onClick={() => void start()} disabled={busy} data-testid="import-pack">
        {busy && !preview ? <Loader2 aria-hidden className="size-4 animate-spin" /> : <Package aria-hidden className="size-4" />}
        {busy && !preview ? t("packs.checking") : t("packs.import")}
      </Button>
      <span className="sr-only" role="status" aria-live="polite">
        {busy && !preview ? t("packs.checking") : done ? t("packs.installed", { name: done.name }) : ""}
      </span>
      <Dialog open={!!preview} onOpenChange={(o) => !o && cancel()}>
        <DialogContent className="max-h-[85vh] overflow-y-auto sm:max-w-lg">
          <DialogHeader>
            <DialogTitle>{t("packs.previewTitle")}</DialogTitle>
            <DialogDescription>{t("packs.previewBody")}</DialogDescription>
          </DialogHeader>
          {preview && <PreviewBody p={preview} />}
          <DialogFooter>
            <Button variant="outline" onClick={cancel}>
              {t("packs.cancel")}
            </Button>
            <Button onClick={() => void apply()} disabled={busy} data-testid="pack-install">
              {preview?.change.kind === "same" ? t("packs.close") : t("packs.install")}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
      <Dialog open={!!error} onOpenChange={(o) => !o && setError(null)}>
        <DialogContent className="max-h-[85vh] overflow-y-auto sm:max-w-lg" data-testid="pack-error">
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2">
              <AlertTriangle aria-hidden className="size-5 text-terracotta-ink" />
              {t("packs.refusedTitle")}
            </DialogTitle>
            <DialogDescription>{error?.summary}</DialogDescription>
          </DialogHeader>
          {error?.detail && (
            <ul className="space-y-1 rounded-md bg-muted/60 p-3 font-mono text-xs" aria-label={t("packs.reasons")}>
              {error.detail.split("\n").map((line, i) => (
                <li key={i} className="break-words">
                  {line}
                </li>
              ))}
            </ul>
          )}
          <DialogFooter>
            <Button onClick={() => setError(null)}>{t("packs.close")}</Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  );
}

/** Settings: installed packs (Uninstall) and whether their problems may be recommended. */
export function PacksSettings() {
  const { t, i18n } = useTranslation();
  const packs = usePacks();
  const refresh = useRefresh();
  const qc = useQueryClient();
  const recommend = useQuery({ queryKey: ["setting", PACKS_RECOMMEND], queryFn: () => api.getSetting(PACKS_RECOMMEND) });
  const [error, setError] = useState<string | null>(null);
  const installed = (packs.data ?? []).filter((p) => !p.removed);
  const toggle = async (on: boolean) => {
    qc.setQueryData(["setting", PACKS_RECOMMEND], on);
    await api.setSetting(PACKS_RECOMMEND, on).catch(() => {});
    void qc.invalidateQueries({ queryKey: queryKeys.progress });
  };
  const uninstall = async (p: PackInfo) => {
    if (!window.confirm(t("packs.uninstallConfirm", { name: p.name }))) return;
    try {
      await api.uninstallPack(p.id);
      refresh();
    } catch (e) {
      setError(describeError(i18n, e).summary);
    }
  };
  return (
    <section aria-labelledby="settings-packs" className="space-y-4 rounded-xl border bg-card p-6" data-testid="packs-settings">
      <h2 id="settings-packs" className="font-display text-2xl font-semibold">
        {t("packs.title")}
      </h2>
      <p className="max-w-2xl text-sm text-muted-foreground">{t("packs.intro")}</p>
      <ImportPackButton />
      {installed.length === 0 ? (
        <p className="text-sm text-muted-foreground">{t("packs.none")}</p>
      ) : (
        <ul className="divide-y rounded-lg border bg-card">
          {installed.map((p) => (
            <li key={p.id} className="flex items-center justify-between gap-3 px-4 py-2 text-sm" data-testid="installed-pack">
              <span>
                <span className="font-semibold">{p.name}</span>{" "}
                <span className="text-muted-foreground">
                  {t("packs.line", { author: p.author, version: p.version, n: p.problems })}
                </span>
              </span>
              <Button variant="ghost" size="sm" className="gap-1" onClick={() => void uninstall(p)} aria-label={t("packs.uninstallNamed", { name: p.name })}>
                <Trash2 aria-hidden className="size-4" />
                {t("packs.uninstall")}
              </Button>
            </li>
          ))}
        </ul>
      )}
      {error && (
        <p role="alert" className="text-sm text-terracotta-ink">
          {error}
        </p>
      )}
      <label className="flex items-center gap-2 text-sm">
        <input type="checkbox" checked={recommend.data === true} onChange={(e) => void toggle(e.target.checked)} data-testid="packs-recommend" />
        {t("packs.recommend")}
      </label>
    </section>
  );
}
