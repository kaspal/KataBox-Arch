// "Save animation (WebM)" (M7b.2): pick a start and a length, then Rust steps the run
// headless and this dialog records it in real time on an offscreen canvas. Opened from the
// Export menu (the Sandbox run or a visible problem test) or from Results ("Export replay"
// of a failed test, inside Replay's window). Rendered once by the Shell.
import { useEffect, useId, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { Film } from "lucide-react";
import { useReportError } from "@/app/dialogs";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { api } from "@/lib/api";
import { useRegistry } from "@/lib/queries";
import { flavourName, kindName } from "@/solve/registry";
import { designSvg, readPalette, svgImage, type FrameWords, type SceneNames } from "@/solve/export/scene";
import { WEBM_MAX_S, recordWebm, renderer, unpackFrames, webmType } from "@/solve/export/animation";
import { resetAnimation, useAnimation } from "@/solve/export/animationStore";

const FPS = 30;

type Phase = { kind: "idle" } | { kind: "preparing" } | { kind: "recording"; done: number; total: number } | { kind: "saving" };

export function AnimationDialog() {
  const { t, i18n } = useTranslation();
  const request = useAnimation((s) => s.request);
  const registry = useRegistry();
  const report = useReportError();
  const [start, setStart] = useState(0);
  const [length, setLength] = useState(10);
  const [phase, setPhase] = useState<Phase>({ kind: "idle" });
  const [outcome, setOutcome] = useState("");
  const stopRef = useRef(false);
  const startId = useId();
  const lengthId = useId();

  useEffect(() => {
    if (!request) return;
    setStart(request.minStart);
    setLength(Math.max(1, Math.min(10, request.durationS - request.minStart)));
    setPhase({ kind: "idle" });
    setOutcome("");
    stopRef.current = false;
  }, [request]);

  // A hidden window throttles timers: stop rather than record a stuttering video.
  useEffect(() => {
    const onHide = () => {
      if (document.hidden) stopRef.current = true;
    };
    document.addEventListener("visibilitychange", onHide);
    return () => document.removeEventListener("visibilitychange", onHide);
  }, []);

  const busy = phase.kind !== "idle";
  const maxLength = request ? Math.max(1, Math.min(WEBM_MAX_S, request.durationS - start)) : WEBM_MAX_S;
  const close = () => {
    stopRef.current = true;
    if (!busy) resetAnimation();
  };

  const run = async () => {
    if (!request || !registry.data) return;
    const type = webmType();
    if (!type) return;
    stopRef.current = false;
    setOutcome("");
    setPhase({ kind: "preparing" });
    try {
      const seconds = Math.max(1, Math.min(length, maxLength));
      const buf = await api.runFrames({ source: request.source, from_s: start, seconds, fps: FPS });
      const { info, frames } = unpackFrames(buf);
      const sceneNames: SceneNames = {
        kind: (k) => kindName(t, k as never),
        flavour: (f) => flavourName(t, f as never),
        protocol: (p) => t(`protocol.${p}` as "protocol.http"),
        zone: (z) => t(`canvas.zone.${z}` as "canvas.zone.region"),
      };
      // Webfonts first (review A7), so text measures the same as on screen.
      await document.fonts?.ready;
      const palette = readPalette();
      const background = await svgImage(designSvg(info.design, registry.data, sceneNames, palette));
      const n0 = new Intl.NumberFormat(i18n.language, { maximumFractionDigits: 0 });
      const n1 = new Intl.NumberFormat(i18n.language, { maximumFractionDigits: 1 });
      const pct = new Intl.NumberFormat(i18n.language, { style: "percent", maximumFractionDigits: 1 });
      const words: FrameWords = {
        stat: (util, queue) => (queue > 0 ? t("canvas.statShortQueued", { util, queue }) : t("canvas.statShortBusy", { util })),
        down: t("canvas.statDown"),
        hud: (f) =>
          t("animation.hud", {
            label: request.label,
            t: n1.format(f.tMs / 1000),
            p95: n1.format(f.p95Ms),
            rps: n0.format(f.rps),
            errors: pct.format(f.errorRate),
          }),
      };
      const r = renderer(frames, info, background, palette, words);
      setPhase({ kind: "recording", done: 0, total: frames.length });
      const bytes = await recordWebm(r, frames.length, FPS, {
        type,
        stop: () => stopRef.current,
        onProgress: (done, total) => setPhase({ kind: "recording", done, total }),
      });
      if (!bytes) {
        setOutcome(document.hidden ? t("animation.hidden") : t("animation.cancelled"));
        setPhase({ kind: "idle" });
        return;
      }
      setPhase({ kind: "saving" });
      const name = `${info.design.name || "design"} - ${request.label}.webm`.replace(/[<>:"/\\|?*]/g, "");
      const saved = await api.saveExport("webm", bytes, name);
      setPhase({ kind: "idle" });
      if (saved) {
        setOutcome(t("animation.saved"));
        resetAnimation();
      }
    } catch (e) {
      setPhase({ kind: "idle" });
      report(e);
    }
  };

  const progress =
    phase.kind === "preparing"
      ? t("animation.preparing")
      : phase.kind === "recording"
        ? t("animation.recording", { done: phase.done, total: phase.total })
        : phase.kind === "saving"
          ? t("animation.saving")
          : outcome;

  return (
    <Dialog open={request !== null} onOpenChange={(open) => !open && close()}>
      <DialogContent className="sm:max-w-md" data-testid="animation-dialog">
        {request && (
          <>
            <DialogHeader>
              <DialogTitle className="flex items-center gap-2">
                <Film aria-hidden className="size-5" />
                {t("animation.title")}
              </DialogTitle>
              <DialogDescription>{request.label}</DialogDescription>
            </DialogHeader>
            <div className="grid grid-cols-2 gap-3">
              <label htmlFor={startId} className="space-y-1 text-sm">
                <span>{t("animation.start")}</span>
                <Input
                  id={startId}
                  type="number"
                  min={request.minStart}
                  max={Math.max(request.minStart, request.durationS - 1)}
                  step={1}
                  value={start}
                  disabled={busy}
                  onChange={(e) => setStart(Math.max(request.minStart, Math.min(Number(e.target.value) || 0, request.durationS - 1)))}
                />
              </label>
              <label htmlFor={lengthId} className="space-y-1 text-sm">
                <span>{t("animation.length", { max: maxLength })}</span>
                <Input
                  id={lengthId}
                  type="number"
                  min={1}
                  max={maxLength}
                  step={1}
                  value={length}
                  disabled={busy}
                  onChange={(e) => setLength(Math.max(1, Math.min(Number(e.target.value) || 1, maxLength)))}
                />
              </label>
            </div>
            <p className="text-xs text-muted-foreground">{t("animation.realTime")}</p>
            {phase.kind === "recording" && (
              <progress className="h-2 w-full" max={phase.total} value={phase.done} aria-label={t("animation.progress")} />
            )}
            <p className="min-h-5 text-sm" role="status" aria-live="polite" data-testid="animation-status">
              {progress}
            </p>
            <DialogFooter>
              {busy ? (
                <Button variant="outline" onClick={() => (stopRef.current = true)}>
                  {t("common.cancel")}
                </Button>
              ) : (
                <>
                  <Button variant="ghost" onClick={close}>
                    {t("common.close")}
                  </Button>
                  <Button onClick={() => void run()} disabled={!registry.data}>
                    {t("animation.record")}
                  </Button>
                </>
              )}
            </DialogFooter>
          </>
        )}
      </DialogContent>
    </Dialog>
  );
}
