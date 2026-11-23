// What a docker-compose import did (M7b.1): mapped, guessed and clamped services, and the
// dependencies it couldn't draw. Rendered once by the Shell; `showComposeSummary` opens it.
import { create } from "zustand";
import { useTranslation } from "react-i18next";
import { CircleCheck, CircleHelp, CircleSlash, Gauge } from "lucide-react";
import type { ComposeImport } from "@/bindings";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { flavourName, kindName } from "@/solve/registry";

const useSummary = create<{ result: ComposeImport | null }>(() => ({ result: null }));

export function showComposeSummary(result: ComposeImport) {
  useSummary.setState({ result });
}

export function resetComposeSummary() {
  useSummary.setState({ result: null });
}

export function ComposeSummary() {
  const { t } = useTranslation();
  const r = useSummary((s) => s.result);
  const close = () => useSummary.setState({ result: null });
  return (
    <Dialog open={r !== null} onOpenChange={(open) => !open && close()}>
      <DialogContent className="max-h-[85vh] overflow-y-auto sm:max-w-lg" data-testid="compose-summary">
        {r && (
          <>
            <DialogHeader>
              <DialogTitle>{t("compose.title")}</DialogTitle>
              <DialogDescription>
                {t("compose.summary", { cards: r.design.nodes.length, edges: r.design.edges.length })}
              </DialogDescription>
            </DialogHeader>
            {r.mapped.length > 0 && (
              <section className="space-y-1" data-testid="compose-mapped">
                <h3 className="flex items-center gap-1.5 text-sm font-semibold">
                  <CircleCheck aria-hidden className="size-4 text-primary" />
                  {t("compose.mapped", { count: r.mapped.length })}
                </h3>
                <ul className="space-y-0.5 text-sm">
                  {r.mapped.map((m) => (
                    <li key={m.service} className="flex justify-between gap-2">
                      <span className="truncate font-mono text-xs">{m.service}</span>
                      <span className="text-muted-foreground">
                        {flavourName(t, m.flavour)} · {kindName(t, m.kind)}
                      </span>
                    </li>
                  ))}
                </ul>
              </section>
            )}
            {r.guessed.length > 0 && (
              <section className="space-y-1" data-testid="compose-guessed">
                <h3 className="flex items-center gap-1.5 text-sm font-semibold">
                  <CircleHelp aria-hidden className="size-4" />
                  {t("compose.guessed", { count: r.guessed.length })}
                </h3>
                <p className="text-xs text-muted-foreground">{t("compose.guessedHint")}</p>
                <ul className="space-y-0.5 text-sm">
                  {r.guessed.map((m) => (
                    <li key={m.service} className="flex justify-between gap-2">
                      <span className="truncate font-mono text-xs">{m.service}</span>
                      <span className="truncate text-muted-foreground">{m.image ?? t("compose.noImage")}</span>
                    </li>
                  ))}
                </ul>
              </section>
            )}
            {r.clamped.length > 0 && (
              <section className="space-y-1" data-testid="compose-clamped">
                <h3 className="flex items-center gap-1.5 text-sm font-semibold">
                  <Gauge aria-hidden className="size-4" />
                  {t("compose.clamped", { count: r.clamped.length })}
                </h3>
                <p className="font-mono text-xs">{r.clamped.join(", ")}</p>
              </section>
            )}
            {r.skipped_edges.length > 0 && (
              <section className="space-y-1" data-testid="compose-skipped">
                <h3 className="flex items-center gap-1.5 text-sm font-semibold">
                  <CircleSlash aria-hidden className="size-4 text-destructive" />
                  {t("compose.skipped", { count: r.skipped_edges.length })}
                </h3>
                <ul className="space-y-1 text-sm">
                  {r.skipped_edges.map((s) => (
                    <li key={`${s.from}>${s.to}`}>
                      <span className="font-mono text-xs">
                        {s.from} → {s.to}
                      </span>
                      <span className="block text-xs text-muted-foreground">
                        {s.reason.type === "unknown_service"
                          ? t("compose.unknownService")
                          : t(`wiring.reason.${s.reason.wiring}` as "wiring.reason.self_loop")}
                      </span>
                    </li>
                  ))}
                </ul>
              </section>
            )}
            <DialogFooter>
              <Button onClick={close} autoFocus>
                {t("compose.done")}
              </Button>
            </DialogFooter>
          </>
        )}
      </DialogContent>
    </Dialog>
  );
}
