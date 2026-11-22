// "Find breaking point": how much traffic the design holds before p95 passes 200 ms or errors
// pass 1%, and the first card to give way. Rust runs the search (ka-sim/src/breaking.rs);
// the answer belongs to the design it was found for and is marked stale after an edit.
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Gauge, Loader2 } from "lucide-react";
import type { BreakingPoint as Result } from "@/bindings";
import { useReportError } from "@/app/dialogs";
import { Button } from "@/components/ui/button";
import { api } from "@/lib/api";
import { flavourName } from "@/solve/registry";
import { simKey } from "@/solve/run/runStore";
import { toDesign, useEditor } from "@/solve/store";

export function BreakingPoint() {
  const { t, i18n } = useTranslation();
  const report = useReportError();
  const [busy, setBusy] = useState(false);
  const [found, setFound] = useState<{ key: string; result: Result } | null>(null);
  const nodes = useEditor((s) => s.doc.nodes);
  const currentKey = useEditor((s) => simKey(toDesign(s)));

  const find = async () => {
    const design = toDesign(useEditor.getState());
    setBusy(true);
    try {
      const result = await api.findBreakingPoint(design);
      setFound({ key: simKey(design), result });
    } catch (e) {
      report(e);
    } finally {
      setBusy(false);
    }
  };

  const n0 = new Intl.NumberFormat(i18n.language, { maximumFractionDigits: 0 });
  const n1 = new Intl.NumberFormat(i18n.language, { maximumFractionDigits: 1 });
  const r = found?.result;
  const first = r?.first_failure;
  const firstNode = first ? nodes.find((n) => n.id === first.node) : undefined;
  const firstName = firstNode ? firstNode.label.trim() || flavourName(t, firstNode.flavour) : first?.node;

  return (
    <section className="space-y-2" data-testid="breaking-point">
      <h3 className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">{t("breaking.title")}</h3>
      {r && (
        <div className="space-y-1 rounded-md border bg-card p-3 text-sm" data-testid="breaking-result">
          <p className="font-medium">
            {r.beyond_ceiling
              ? t("breaking.holdsBeyond", { rps: n0.format(r.limit_rps) })
              : t("breaking.holds", { rps: n0.format(r.limit_rps), x: n1.format(r.multiplier) })}
          </p>
          {first ? (
            <p>
              {t(`breaking.reason.${first.reason}`, { name: firstName, at: n0.format(first.at_s + 1) })}
            </p>
          ) : (
            !r.beyond_ceiling && <p>{t("breaking.latencyOnly", { p95: n0.format(r.failing_p95_ms) })}</p>
          )}
          {r.stale_rate > 0 && (
            <p className="text-muted-foreground">
              {t("breaking.staleReads", { pct: new Intl.NumberFormat(i18n.language, { style: "percent", maximumFractionDigits: 2 }).format(r.stale_rate) })}
            </p>
          )}
          {found && found.key !== currentKey && <p className="text-xs text-muted-foreground">{t("breaking.stale")}</p>}
        </div>
      )}
      <Button variant="outline" size="sm" className="gap-2" onClick={() => void find()} disabled={busy}>
        {busy ? <Loader2 aria-hidden className="size-4 animate-spin motion-reduce:animate-none" /> : <Gauge aria-hidden className="size-4" />}
        {busy ? t("breaking.finding") : t("breaking.find")}
      </Button>
      <p className="text-xs text-muted-foreground">{t("breaking.limits")}</p>
    </section>
  );
}
