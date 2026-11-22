// The Run controls in the Sandbox toolbar: the traffic preset and Run when idle; Inject (chaos
// at the current moment) and Stop while a run is on. Play/pause, the clock, the scrubber and
// the speed live in the timeline under the canvas.
import { useTranslation } from "react-i18next";
import { Play, RotateCcw, Square, Zap } from "lucide-react";
import type { ChaosEvent, SandboxPreset } from "@/bindings";
import { useReportError } from "@/app/dialogs";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { flavourName } from "@/solve/registry";
import { chaos } from "@/solve/run/chaos";
import { useProblemMode } from "@/solve/problem/context";
import { isActive, useRun } from "@/solve/run/runStore";
import { toDesign, useEditor } from "@/solve/store";

const PRESETS: SandboxPreset[] = ["normal", "spike", "kill_node"];

function InjectMenu() {
  const { t } = useTranslation();
  const report = useReportError();
  const status = useRun((s) => s.status);
  const selected = useEditor((s) => (s.selection.length === 1 ? s.doc.nodes.find((n) => n.id === s.selection[0]) : undefined));
  const name = selected ? selected.label.trim() || flavourName(t, selected.flavour) : "";
  const inject = (event: ChaosEvent) => {
    useRun.getState().inject(toDesign(useEditor.getState()), event).catch(report);
  };
  const cache = selected?.kind === "cache" || selected?.kind === "cdn";
  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button variant="outline" size="sm" className="gap-2" disabled={status === "starting"}>
          <Zap aria-hidden className="size-4" />
          {t("run.inject")}
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="w-64">
        <DropdownMenuLabel>{selected ? t("run.injectOn", { name }) : t("run.injectSelect")}</DropdownMenuLabel>
        <DropdownMenuItem disabled={!selected} onSelect={() => selected && inject(chaos.kill(selected.id))}>
          {t("run.injectKill")}
        </DropdownMenuItem>
        <DropdownMenuItem disabled={!selected} onSelect={() => selected && inject(chaos.latency(selected.id, 200, 10))}>
          {t("run.injectSlow")}
        </DropdownMenuItem>
        <DropdownMenuItem disabled={!cache} onSelect={() => selected && inject(chaos.flush(selected.id))}>
          {t("run.injectFlush")}
        </DropdownMenuItem>
        <DropdownMenuSeparator />
        <DropdownMenuItem onSelect={() => inject(chaos.traffic(3, 10))}>{t("run.injectTraffic")}</DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

export function RunControls() {
  const { t } = useTranslation();
  const report = useReportError();
  const status = useRun((s) => s.status);
  const preset = useRun((s) => s.preset);
  const problemTest = useRun((s) => s.problemTest);
  const problem = useProblemMode();
  const { i18n } = useTranslation();
  const run = useRun.getState();

  const start = () => {
    run.start(toDesign(useEditor.getState())).catch(report);
  };

  if (isActive(status)) {
    return (
      <div className="flex items-center gap-2" role="group" aria-label={t("run.controls")}>
        <InjectMenu />
        <Button variant="outline" size="sm" className="gap-2" onClick={run.stop}>
          <Square aria-hidden className="size-4" />
          {t("run.stop")}
        </Button>
      </div>
    );
  }
  return (
    <div className="flex items-center gap-2" role="group" aria-label={t("run.controls")}>
      {problem ? (
        <Select
          value={problemTest?.test_id ?? ""}
          onValueChange={(v) => run.setProblemTest({ problem_id: problem.id, test_id: v })}
        >
          <SelectTrigger size="sm" className="w-44 bg-card" aria-label={t("run.test")}>
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {problem.brief.tests
              .filter((x) => !x.hidden || x.id === problemTest?.test_id)
              .map((x) => (
                <SelectItem key={x.id} value={x.id}>
                  {i18n.language.startsWith("es") ? x.name_es : x.name}
                </SelectItem>
              ))}
          </SelectContent>
        </Select>
      ) : (
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
      )}
      <Button variant="outline" size="sm" className="gap-2" onClick={start}>
        {status === "finished" ? <RotateCcw aria-hidden className="size-4" /> : <Play aria-hidden className="size-4" />}
        {status === "finished" ? t("run.runAgain") : t("sandbox.run")}
      </Button>
    </div>
  );
}
