// The component library (left panel): every kind as a row with its own icon tile, grouped
// by category with collapsible headers and counts, plus zones and notes. Rows drag onto
// the canvas; Enter adds at the viewport centre (both open the flavour picker).
import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { ChevronDown, ChevronRight, Info, Search } from "lucide-react";
import type { Category, KindSpec, MetaKind, Registry } from "@/bindings";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { normalizeForSearch } from "@/app/paletteFilter";
import { cn } from "@/lib/utils";
import { requestAdd, type AddRequest } from "@/solve/bridge";
import { KIND_MIME } from "@/solve/Canvas";
import { Glyph, GlyphTile } from "@/solve/icons";
import { categoryName, flavourName, kindName } from "@/solve/registry";

type Row = { key: string; label: string; icon: string; category: Category | "meta"; req: AddRequest; spec?: KindSpec };

const META_ICON: Record<MetaKind, string> = { region: "zone", az: "zone", note: "note" };

function startDrag(e: React.DragEvent, req: AddRequest) {
  e.dataTransfer.setData(KIND_MIME, JSON.stringify(req));
  e.dataTransfer.effectAllowed = "copy";
}

export function LibraryPanel({ registry }: { registry: Registry }) {
  const { t } = useTranslation();
  const [query, setQuery] = useState("");
  const [collapsed, setCollapsed] = useState<Set<string>>(new Set());
  const [quickLook, setQuickLook] = useState<KindSpec | null>(null);

  const groups = useMemo(() => {
    const q = normalizeForSearch(query);
    const matches = (texts: string[]) => q === "" || texts.some((s) => normalizeForSearch(s).includes(q));
    const out: { id: string; title: string; rows: Row[] }[] = registry.categories.map((c) => ({
      id: c,
      title: categoryName(t, c),
      rows: registry.kinds
        .filter((k) => k.category === c)
        .filter((k) => matches([kindName(t, k.kind), ...k.flavours.map((f) => flavourName(t, f.flavour))]))
        .map((k) => ({
          key: k.kind,
          label: kindName(t, k.kind),
          icon: k.icon,
          category: k.category,
          req: { kind: k.kind },
          spec: k,
        })),
    }));
    const meta: Row[] = registry.meta
      .filter((m) => matches([t(`canvas.zone.${m.kind === "note" ? "note" : m.kind}`)]))
      .map((m) => ({
        key: m.kind,
        label: t(`canvas.zone.${m.kind === "note" ? "note" : m.kind}`),
        icon: META_ICON[m.kind],
        category: "meta",
        req: { meta: m.kind },
      }));
    out.push({ id: "meta", title: t("library.meta"), rows: meta });
    return out.filter((g) => g.rows.length > 0);
  }, [query, registry, t]);

  const toggle = (id: string) =>
    setCollapsed((s) => {
      const next = new Set(s);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });

  return (
    <aside aria-label={t("library.title")} className="flex h-full min-h-0 flex-col bg-background" data-testid="library">
      <div className="flex items-baseline justify-between px-4 pb-2 pt-4">
        <h2 className="text-base font-semibold">{t("library.title")}</h2>
        <span className="text-xs text-muted-foreground">{t("library.hint")}</span>
      </div>
      <div className="relative px-4 pb-3">
        <Search aria-hidden className="pointer-events-none absolute left-6.5 top-2.5 size-4 text-muted-foreground" />
        <Input
          type="search"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          placeholder={t("library.search")}
          aria-label={t("library.searchLabel")}
          className="bg-card pl-8"
        />
      </div>
      <div className="min-h-0 flex-1 overflow-y-auto px-2 pb-4">
        {groups.length === 0 && <p className="px-2 py-4 text-sm text-muted-foreground">{t("library.noMatch")}</p>}
        {groups.map((g) => {
          const open = query !== "" || !collapsed.has(g.id);
          const headId = `library-${g.id}`;
          return (
            <section key={g.id} aria-labelledby={headId} className="mb-1">
              <h3 id={headId}>
                <button
                  type="button"
                  className="flex w-full items-center justify-between rounded-md px-2 py-1.5 text-xs font-medium text-muted-foreground hover:bg-accent"
                  aria-expanded={open}
                  onClick={() => toggle(g.id)}
                >
                  <span>
                    {g.title} · {g.rows.length}
                  </span>
                  {open ? <ChevronDown aria-hidden className="size-3.5" /> : <ChevronRight aria-hidden className="size-3.5" />}
                </button>
              </h3>
              {open && (
                <ul className="space-y-0.5">
                  {g.rows.map((r) => (
                    <li key={r.key} className="group flex items-center gap-1">
                      <div
                        role="button"
                        tabIndex={0}
                        draggable
                        onDragStart={(e) => startDrag(e, r.req)}
                        onKeyDown={(e) => {
                          if (e.key === "Enter" || e.key === " ") {
                            e.preventDefault();
                            requestAdd({ ...r.req, pick: true });
                          }
                        }}
                        onDoubleClick={() => requestAdd({ ...r.req, pick: true })}
                        aria-label={t("library.add", { name: r.label })}
                        className={cn(
                          "flex min-w-0 flex-1 cursor-grab items-center gap-2.5 rounded-md px-2 py-1.5 text-sm hover:bg-accent active:cursor-grabbing",
                        )}
                        data-testid="library-row"
                        data-kind={r.key}
                      >
                        {r.category === "meta" ? (
                          <span className="flex size-7 shrink-0 items-center justify-center rounded-md bg-muted text-muted-foreground" aria-hidden>
                            <Glyph icon={r.icon} />
                          </span>
                        ) : (
                          <GlyphTile icon={r.icon} category={r.category} />
                        )}
                        <span className="truncate">{r.label}</span>
                      </div>
                      {r.spec && (
                        <Button
                          variant="ghost"
                          size="icon"
                          className="size-7 text-muted-foreground opacity-60 group-hover:opacity-100 focus-visible:opacity-100"
                          aria-label={t("library.quickLook", { name: r.label })}
                          onClick={() => setQuickLook(r.spec!)}
                        >
                          <Info aria-hidden className="size-3.5" />
                        </Button>
                      )}
                    </li>
                  ))}
                </ul>
              )}
            </section>
          );
        })}
      </div>
      <QuickLook spec={quickLook} onClose={() => setQuickLook(null)} />
    </aside>
  );
}

/** Placeholder quick look; the full one (concept, when to use, numbers) arrives in M3. */
function QuickLook({ spec, onClose }: { spec: KindSpec | null; onClose: () => void }) {
  const { t } = useTranslation();
  return (
    <Dialog open={spec !== null} onOpenChange={(o) => !o && onClose()}>
      <DialogContent data-testid="quick-look">
        {spec && (
          <>
            <DialogHeader>
              <div className="flex items-center gap-3">
                <GlyphTile icon={spec.icon} category={spec.category} className="size-10 [&_svg]:size-6" />
                <div>
                  <DialogTitle className="font-display text-2xl">{kindName(t, spec.kind)}</DialogTitle>
                  <DialogDescription>{categoryName(t, spec.category)}</DialogDescription>
                </div>
              </div>
            </DialogHeader>
            <div className="space-y-2 text-sm">
              <h4 className="font-medium">{t("library.flavours")}</h4>
              <ul className="flex flex-wrap gap-1.5">
                {spec.flavours.map((f) => (
                  <li key={f.flavour} className="rounded-full border bg-muted px-2.5 py-0.5 text-xs">
                    {flavourName(t, f.flavour)}
                  </li>
                ))}
              </ul>
              <p className="pt-2 text-muted-foreground">{t("library.guideLater")}</p>
            </div>
          </>
        )}
      </DialogContent>
    </Dialog>
  );
}
