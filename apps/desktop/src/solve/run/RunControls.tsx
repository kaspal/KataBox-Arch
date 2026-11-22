// The Run controls in the Sandbox toolbar: preset and Run when idle; pause/resume, stop,
// speed and the simulated clock while a run is on.
import { useTranslation } from "react-i18next";
import { Loader2, Pause, Play, RotateCcw, Square } from "lucide-react";
import type { SandboxPreset } from "@/bindings";
import { useReportError } from "@/app/dialogs";
import { Button } from "@/components/ui/button";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { toDesign, useEditor } from "@/solve/store";
import { SPEEDS, useRun } from "@/solve/run/runStore";

const PRESETS: SandboxPreset[] = ["normal", "spike"];

export function RunControls() {
  const { t } = useTranslation();
  const report = useReportError();
  const status = useRun((s) => s.status);
  const preset = useRun((s) => s.preset);
  const speed = useRun((s) => s.speed);
  const tMs = useRun((s) => s.frame?.tMs ?? 0);
  const duration = useRun((s) => s.info?.duration_s ?? 0);
  const run = useRun.getState();

  const start = () => {
    run.start(toDesign(useEditor.getState())).catch(report);
  };

  if (status === "idle" || status === "finished") {
    return (
      <div className="flex items-center gap-2" role="group" aria-label={t("run.controls")}>
        <Select value={preset} onValueChange={(v) => run.setPreset(v as SandboxPreset)}>
          <SelectTrigger size="sm" className="w-40 bg-card" aria-label={t("run.preset")}>
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {PRESETS.map((p) => (
              <SelectItem key={p} value={p}>
                {t(`run.presets.${p}`)}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
        <Button variant="outline" size="sm" className="gap-2" onClick={start}>
          {status === "finished" ? <RotateCcw aria-hidden className="size-4" /> : <Play aria-hidden className="size-4" />}
          {status === "finished" ? t("run.runAgain") : t("sandbox.run")}
        </Button>
      </div>
    );
  }

  const seconds = (tMs / 1000).toFixed(1);
  return (
    <div className="flex items-center gap-2" role="group" aria-label={t("run.controls")}>
      <span className="whitespace-nowrap font-mono text-xs tabular-nums text-muted-foreground" data-testid="run-clock">
        {status === "starting" ? t("run.starting") : t("run.clock", { t: seconds, total: duration })}
      </span>
      <Select value={String(speed)} onValueChange={(v) => run.setSpeed(Number(v))}>
        <SelectTrigger size="sm" className="w-20 bg-card" aria-label={t("run.speed")}>
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          {SPEEDS.map((s) => (
            <SelectItem key={s} value={String(s)}>
              {`${s}×`}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
      {status === "starting" ? (
        <Button variant="outline" size="sm" className="gap-2" disabled>
          <Loader2 aria-hidden className="size-4 animate-spin motion-reduce:animate-none" />
          {t("run.starting")}
        </Button>
      ) : status === "playing" ? (
        <Button variant="outline" size="sm" className="gap-2" onClick={run.pause}>
          <Pause aria-hidden className="size-4" />
          {t("run.pause")}
        </Button>
      ) : (
        <Button variant="outline" size="sm" className="gap-2" onClick={run.resume}>
          <Play aria-hidden className="size-4" />
          {t("run.resume")}
        </Button>
      )}
      <Button variant="ghost" size="icon" className="size-8" aria-label={t("run.stop")} onClick={run.stop}>
        <Square aria-hidden className="size-4" />
      </Button>
    </div>
  );
}
