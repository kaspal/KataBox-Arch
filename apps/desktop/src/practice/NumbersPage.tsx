// /numbers: the numbers cheat sheet (M5.1): rough speeds and sizes for estimates, grouped and
// searchable. Each figure says where it comes from: a vendor page, the simulator's own
// calibration, plain arithmetic, or a source still to be checked.
import { useState } from "react";
import type { TFunction } from "i18next";
import { useTranslation } from "react-i18next";
import { Calculator, Search } from "lucide-react";
import type { NumberFigure, NumberGroup } from "@/bindings";
import { EmptyState, Page } from "@/components/Page";
import { Input } from "@/components/ui/input";
import { Skeleton } from "@/components/ui/skeleton";
import { describeError } from "@/lib/errors";
import { useNumbers } from "@/lib/queries";

const GROUPS: NumberGroup[] = ["memory", "storage", "network", "servers", "sizes", "time"];

/** How a figure's source reads: a host name for a page, else what kind of figure it is. */
export function sourceLabel(t: TFunction, source: string): string {
  const s = source.trim();
  if (s === "arithmetic") return t("numbers.source.arithmetic");
  if (s === "simulator") return t("numbers.source.simulator");
  const needed = s.startsWith("needed:");
  const url = needed ? s.slice("needed:".length).trim() : s;
  let host = url;
  try {
    host = new URL(url).hostname.replace(/^www\./, "");
  } catch {
    /* shown as written */
  }
  return needed ? t("numbers.source.needed", { host }) : t("numbers.source.cited", { host });
}

export function formatFigure(lang: string, f: Pick<NumberFigure, "value" | "unit">, countWord: string): string {
  const n = new Intl.NumberFormat(lang, { maximumFractionDigits: f.value < 1 ? 3 : f.value < 10 ? 1 : 0 }).format(f.value);
  if (f.unit === "count") return n;
  if (f.unit === "rps") return `${n} ${countWord}`;
  return `${n} ${f.unit === "us" ? "µs" : f.unit}`;
}

export function NumbersPage() {
  const { t, i18n } = useTranslation();
  const numbers = useNumbers();
  const [q, setQ] = useState("");
  const es = i18n.language.startsWith("es");
  if (numbers.error) {
    return (
      <Page title={t("numbers.title")}>
        <EmptyState icon={Calculator} title={t("guide.unavailable")} body={describeError(i18n, numbers.error).summary} />
      </Page>
    );
  }
  if (!numbers.data) {
    return (
      <Page title={t("numbers.title")}>
        <Skeleton className="h-80 w-full" />
      </Page>
    );
  }
  const needle = q.trim().toLowerCase();
  const shown = numbers.data.filter((f) => !needle || `${f.label} ${f.label_es} ${f.note} ${f.note_es}`.toLowerCase().includes(needle));
  return (
    <Page
      title={t("numbers.title")}
      actions={
        <label className="relative block w-72">
          <Search aria-hidden className="pointer-events-none absolute top-1/2 left-3 size-4 -translate-y-1/2 text-muted-foreground" />
          <Input type="search" value={q} onChange={(e) => setQ(e.target.value)} placeholder={t("numbers.search")} aria-label={t("numbers.search")} className="bg-card pl-9" />
        </label>
      }
    >
      <p className="mb-6 max-w-2xl text-sm text-muted-foreground">{t("numbers.intro")}</p>
      {shown.length === 0 ? (
        <p className="text-muted-foreground">{t("guide.noMatch")}</p>
      ) : (
        <div className="space-y-8" data-testid="numbers">
          {GROUPS.map((g) => {
            const rows = shown.filter((f) => f.group === g);
            if (rows.length === 0) return null;
            const head = `numbers-${g}`;
            return (
              <section key={g} aria-labelledby={head} className="space-y-2">
                <h2 id={head} className="font-display text-xl font-semibold">
                  {t(`numbers.group.${g}`)}
                </h2>
                <dl className="divide-y rounded-xl border bg-card">
                  {rows.map((f) => (
                    <div key={f.id} id={f.id} className="grid gap-1 px-5 py-3 sm:grid-cols-[16rem_9rem_1fr] sm:gap-6">
                      <dt className="font-semibold">{es ? f.label_es : f.label}</dt>
                      <dd className="font-mono text-sm tabular-nums" data-testid="figure-value">
                        {formatFigure(i18n.language, f, t("units.rps"))}
                      </dd>
                      <dd className="text-sm">
                        {es ? f.note_es : f.note}
                        <span className="mt-1 block text-xs text-muted-foreground">{sourceLabel(t, f.source)}</span>
                      </dd>
                    </div>
                  ))}
                </dl>
              </section>
            );
          })}
        </div>
      )}
    </Page>
  );
}
