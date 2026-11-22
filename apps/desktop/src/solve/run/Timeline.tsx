// The run's timeline under the canvas: play/pause, the simulated clock, a scrubber with a
// diamond for each chaos event, and the speed. Seeking re-simulates from the nearest
// snapshot in Rust (deterministic) and pauses there.
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Pause, Play } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { chaosKind } from "@/solve/run/chaos";
import { chaosTime, isActive, SPEEDS, useRun } from "@/solve/run/runStore";

export function Timeline() {
  const { t } = useTranslation();
  const status = useRun((s) => s.status);
  const tMs = useRun((s) => s.frame?.tMs ?? 0);
  const info = useRun((s) => s.info);
  const speed = useRun((s) => s.speed);
  const run = useRun.getState();
  // While dragging, the thumb follows the pointer; the seek happens on release.
  const [drag, setDrag] = useState<number | null>(null);
  useEffect(() => {
    if (status === "idle") setDrag(null);
  }, [status]);
  if (!(isActive(status) || status === "finished") || !info) return null;
  const duration = info.duration_s;
  const now = drag ?? tMs / 1000;
  const commit = () => {
    if (drag !== null) run.seek(drag);
    setDrag(null);
  };
  return (
    <div className="flex items-center gap-3 border-t bg-background px-3 py-2" role="group" aria-label={t("run.timeline")} data-testid="timeline">
      {status === "playing" ? (
        <Button variant="outline" size="sm" className="gap-2" onClick={run.pause}>
          <Pause aria-hidden className="size-4" />
          {t("run.pause")}
        </Button>
      ) : (
        <Button variant="outline" size="sm" className="gap-2" onClick={run.resume} disabled={status === "starting" || status === "finished"}>
          <Play aria-hidden className="size-4" />
          {t("run.resume")}
        </Button>
      )}
      <span className="w-28 shrink-0 whitespace-nowrap font-mono text-xs tabular-nums text-muted-foreground" data-testid="run-clock">
        {status === "starting" ? t("run.starting") : t("run.clock", { t: now.toFixed(1), total: duration })}
      </span>
      <div className="relative flex-1">
        <input
          type="range"
          min={0}
          max={duration}
          step={0.1}
          value={now}
          aria-label={t("run.scrub")}
          className="w-full accent-[var(--primary)]"
          onChange={(e) => setDrag(Number(e.target.value))}
          onPointerUp={commit}
          onKeyUp={commit}
          onBlur={commit}
        />
        {info.chaos.map((e, i) => {
          const at = chaosTime(e);
          const label = t(`run.chaos.${chaosKind(e)}`, { at: at.toFixed(1) });
          return (
            <Tooltip key={i}>
              <TooltipTrigger asChild>
                <span
                  className="absolute -top-1.5 size-2.5 -translate-x-1/2 rotate-45 border border-background bg-destructive"
                  style={{ left: `${(at / duration) * 100}%` }}
                  role="img"
                  aria-label={label}
                  data-testid="chaos-marker"
                />
              </TooltipTrigger>
              <TooltipContent>{label}</TooltipContent>
            </Tooltip>
          );
        })}
      </div>
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
    </div>
  );
}
