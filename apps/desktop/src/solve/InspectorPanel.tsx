// The Inspector (right panel): edits what is selected. Card props are generated from the
// registry's PropSpecs (ranges, steps and units come from Rust); a value outside its range
// is shown as an error and never stored.
import { useEffect, useId, useState } from "react";
import { useTranslation } from "react-i18next";
import { Link } from "@tanstack/react-router";
import { Trash2, BookOpen, RotateCcw } from "lucide-react";
import type { Edge, Flavour, Group, Node, Note, PropSpec, PropValue, Protocol, Registry, Scaling } from "@/bindings";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { GlyphTile } from "@/solve/icons";
import { choiceName, defaultProps, flavourName, kindName, kindSpec, propName, unitName } from "@/solve/registry";
import { BriefPanel } from "@/solve/problem/BriefPanel";
import { useProblemMode } from "@/solve/problem/context";
import { MetricsPanel } from "@/solve/run/MetricsPanel";
import { useRun } from "@/solve/run/runStore";
import { useEditor } from "@/solve/store";
import { checkWire, makeWiring } from "@/solve/wiring";

function Field({
  label,
  htmlFor,
  children,
  error,
  tag,
  action,
}: {
  label: string;
  htmlFor: string;
  children: React.ReactNode;
  error?: string;
  /** A short marker after the label, e.g. "ballpark" for a calibrated default. */
  tag?: { text: string; title: string };
  /** A small control beside the label (e.g. reset to default). */
  action?: React.ReactNode;
}) {
  return (
    <div className="space-y-1">
      <div className="flex items-center justify-between gap-2">
        <label htmlFor={htmlFor} className="text-xs font-medium text-muted-foreground">
          {label}
        </label>
        <span className="flex items-center gap-1">
          {action}
          {tag && (
            <span title={tag.title} data-testid="calibration-tag" className="rounded border px-1 text-[10px] leading-4 text-muted-foreground">
              {tag.text}
            </span>
          )}
        </span>
      </div>
      {children}
      {error && (
        <p className="text-xs text-destructive" role="alert">
          {error}
        </p>
      )}
    </div>
  );
}

/** A text field that commits on blur or Enter. */
function TextField({
  label,
  value,
  max,
  onCommit,
  multiline,
  placeholder,
}: {
  label: string;
  value: string;
  max: number;
  onCommit: (v: string) => void;
  multiline?: boolean;
  placeholder?: string;
}) {
  const id = useId();
  const [draft, setDraft] = useState(value);
  useEffect(() => setDraft(value), [value]);
  const commit = () => draft !== value && onCommit(draft);
  return (
    <Field label={label} htmlFor={id}>
      {multiline ? (
        <textarea
          id={id}
          value={draft}
          maxLength={max}
          rows={5}
          onChange={(e) => setDraft(e.target.value)}
          onBlur={commit}
          className="w-full rounded-md border bg-card px-3 py-2 font-hand text-base outline-none focus-visible:ring-2 focus-visible:ring-ring"
        />
      ) : (
        <Input
          id={id}
          value={draft}
          maxLength={max}
          placeholder={placeholder}
          onChange={(e) => setDraft(e.target.value)}
          onBlur={commit}
          onKeyDown={(e) => e.key === "Enter" && commit()}
          className="bg-card"
        />
      )}
    </Field>
  );
}

/**
 * One prop. Cards store only the props the user set (`.kbarch` v3): `value` is the card's own
 * value, `fallback` the flavour's default shown when it has none. A set value can be reset.
 */
function PropField({
  spec,
  value,
  fallback,
  onChange,
  onReset,
}: {
  spec: PropSpec;
  value: PropValue | undefined;
  fallback: PropValue | undefined;
  onChange: (v: PropValue) => void;
  onReset?: () => void;
}) {
  const { t } = useTranslation();
  const id = useId();
  const byDefault = fallback ?? spec.default;
  const current = value ?? byDefault;
  // Plain counts and unitless props show no suffix.
  const unit = spec.unit === "none" || spec.unit === "count" ? "" : unitName(t, spec.unit);
  const label = propName(t, spec.key);
  const tag = spec.calibration
    ? spec.calibration.status === "cited"
      ? { text: t("inspector.cited"), title: t("inspector.citedHint") }
      : { text: t("inspector.ballpark"), title: t("inspector.ballparkHint") }
    : undefined;
  const [draft, setDraft] = useState(String(current));
  const [error, setError] = useState<string | undefined>();
  useEffect(() => {
    setDraft(String(value ?? byDefault));
    setError(undefined);
  }, [value, byDefault]);
  const shown = (v: PropValue) => (typeof v === "string" ? choiceName(t, v) : typeof v === "boolean" ? (v ? t("guide.yes") : t("guide.no")) : String(v));
  const reset =
    value !== undefined && onReset ? (
      <button
        type="button"
        onClick={onReset}
        data-testid="prop-reset"
        title={t("inspector.resetHint", { value: shown(byDefault) })}
        aria-label={t("inspector.reset", { prop: label })}
        className="rounded p-0.5 text-muted-foreground hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
      >
        <RotateCcw aria-hidden className="size-3" />
      </button>
    ) : undefined;

  const ty = spec.ty;
  if (ty.type === "bool") {
    return (
      <label className="flex items-center justify-between gap-3 py-1 text-sm">
        <span>{label}</span>
        <span className="flex items-center gap-2">
          {reset}
          <input type="checkbox" className="size-4 accent-[var(--primary)]" checked={current === true} onChange={(e) => onChange(e.target.checked)} />
        </span>
      </label>
    );
  }
  if (ty.type === "choice") {
    return (
      <Field label={label} htmlFor={id} action={reset}>
        <Select value={String(current)} onValueChange={(v) => onChange(v)}>
          <SelectTrigger id={id} className="w-full bg-card">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {ty.options.map((o) => (
              <SelectItem key={o} value={o}>
                {choiceName(t, o)}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </Field>
    );
  }
  const commit = () => {
    const n = Number(draft);
    const whole = ty.type === "int";
    if (draft.trim() === "" || !Number.isFinite(n) || n < ty.min || n > ty.max || (whole && !Number.isInteger(n))) {
      setError(t(whole ? "inspector.rangeInt" : "inspector.range", { min: ty.min, max: ty.max }));
      return;
    }
    setError(undefined);
    if (n !== current) onChange(n);
  };
  return (
    <Field label={unit ? `${label} (${unit})` : label} htmlFor={id} error={error} tag={tag} action={reset}>
      <Input
        id={id}
        type="number"
        inputMode={ty.type === "int" ? "numeric" : "decimal"}
        min={ty.min}
        max={ty.max}
        step={ty.step}
        value={draft}
        aria-invalid={error ? true : undefined}
        onChange={(e) => setDraft(e.target.value)}
        onBlur={commit}
        onKeyDown={(e) => e.key === "Enter" && commit()}
        className="bg-card font-mono"
      />
    </Field>
  );
}

function DeleteButton({ ids }: { ids: string[] }) {
  const { t } = useTranslation();
  const removeItems = useEditor((s) => s.removeItems);
  return (
    <Button variant="outline" size="sm" className="gap-2 text-destructive" onClick={() => removeItems(ids)}>
      <Trash2 aria-hidden className="size-4" />
      {t("inspector.delete")}
    </Button>
  );
}

function NodeInspector({ registry, node }: { registry: Registry; node: Node }) {
  const { t } = useTranslation();
  const spec = kindSpec(registry, node.kind);
  const updateNode = useEditor((s) => s.updateNode);
  const setProp = useEditor((s) => s.setProp);
  const resetProp = useEditor((s) => s.resetProp);
  const setFlavour = useEditor((s) => s.setFlavour);
  const flavourId = useId();
  const defaults = defaultProps(spec, node.flavour);
  return (
    <div className="space-y-4" data-testid="inspector-node">
      <div className="flex items-center gap-3">
        <GlyphTile icon={spec.icon} category={spec.category} className="size-9 [&_svg]:size-5" />
        <div className="min-w-0">
          <div className="truncate font-semibold">{node.label.trim() || flavourName(t, node.flavour)}</div>
          <div className="text-xs text-muted-foreground">{kindName(t, node.kind)}</div>
        </div>
        <Link
          to="/guide/$kind"
          params={{ kind: node.kind }}
          search={{ section: "sim" }}
          className="ml-auto flex shrink-0 items-center gap-1 text-xs font-semibold text-primary underline-offset-4 hover:underline"
        >
          <BookOpen aria-hidden className="size-3.5" />
          {t("inspector.learnMore")}
        </Link>
      </div>
      <TextField
        label={t("inspector.label")}
        value={node.label}
        max={registry.limits.max_label}
        placeholder={flavourName(t, node.flavour)}
        onCommit={(label) => updateNode(node.id, { label })}
      />
      <Field label={t("inspector.flavour")} htmlFor={flavourId}>
        <Select value={node.flavour} onValueChange={(f) => setFlavour(registry, node.id, f as Node["flavour"])}>
          <SelectTrigger id={flavourId} className="w-full bg-card">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {spec.flavours.map((f) => (
              <SelectItem key={f.flavour} value={f.flavour}>
                {flavourName(t, f.flavour)}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </Field>
      <div className="space-y-3 border-t pt-4">
        <h3 className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">{t("inspector.props")}</h3>
        {spec.props.map((p) => (
          <PropField
            key={p.key}
            spec={p}
            value={node.props[p.key]}
            fallback={defaults[p.key]}
            onChange={(v) => setProp(node.id, p.key, v)}
            onReset={() => resetProp(node.id, p.key)}
          />
        ))}
        {spec.props.some((p) => p.calibration) && <p className="text-xs text-muted-foreground">{t("inspector.calibrationNote")}</p>}
      </div>
      <DeleteButton ids={[node.id]} />
    </div>
  );
}

function GroupInspector({ registry, group }: { registry: Registry; group: Group }) {
  const { t } = useTranslation();
  const updateGroup = useEditor((s) => s.updateGroup);
  return (
    <div className="space-y-4" data-testid="inspector-zone">
      <div className="font-semibold">{t(`canvas.zone.${group.kind}`)}</div>
      <TextField
        label={t("inspector.label")}
        value={group.label}
        max={registry.limits.max_label}
        onCommit={(label) => updateGroup(group.id, { label })}
      />
      {group.scaling ? (
        <ScalingFields registry={registry} group={group} scaling={group.scaling} />
      ) : (
        <p className="text-xs text-muted-foreground">{t("inspector.zoneHint")}</p>
      )}
      <DeleteButton ids={[group.id]} />
    </div>
  );
}

/** An autoscaling wrapper's policy: the Autoscaling kind's flavour and props. */
function ScalingFields({ registry, group, scaling }: { registry: Registry; group: Group; scaling: Scaling }) {
  const { t } = useTranslation();
  const updateScaling = useEditor((s) => s.updateScaling);
  const spec = kindSpec(registry, "autoscale");
  const flavourId = useId();
  const defaults = defaultProps(spec, scaling.flavour);
  return (
    <div className="space-y-3 border-t pt-4">
      <h3 className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">{t("inspector.scaling")}</h3>
      <Field label={t("inspector.flavour")} htmlFor={flavourId}>
        <Select value={scaling.flavour} onValueChange={(f) => updateScaling(group.id, { ...scaling, flavour: f as Flavour })}>
          <SelectTrigger id={flavourId} className="w-full bg-card">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {spec.flavours.map((f) => (
              <SelectItem key={f.flavour} value={f.flavour}>
                {flavourName(t, f.flavour)}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </Field>
      {spec.props.map((p) => (
        <PropField
          key={p.key}
          spec={p}
          value={scaling.props[p.key]}
          fallback={defaults[p.key]}
          onChange={(v) => updateScaling(group.id, { ...scaling, props: { ...scaling.props, [p.key]: v } })}
          onReset={() => {
            const props = { ...scaling.props };
            delete props[p.key];
            updateScaling(group.id, { ...scaling, props });
          }}
        />
      ))}
      <p className="text-xs text-muted-foreground">{t("inspector.autoscaleHint")}</p>
    </div>
  );
}

function NoteInspector({ registry, note }: { registry: Registry; note: Note }) {
  const { t } = useTranslation();
  const updateNote = useEditor((s) => s.updateNote);
  return (
    <div className="space-y-4" data-testid="inspector-note">
      <TextField
        label={t("canvas.noteText")}
        value={note.text}
        max={registry.limits.max_note}
        multiline
        onCommit={(text) => updateNote(note.id, { text })}
      />
      <DeleteButton ids={[note.id]} />
    </div>
  );
}

function EdgeInspector({ registry, edge }: { registry: Registry; edge: Edge }) {
  const { t } = useTranslation();
  const doc = useEditor((s) => s.doc);
  const setEdgeProtocol = useEditor((s) => s.setEdgeProtocol);
  const id = useId();
  const from = doc.nodes.find((n) => n.id === edge.from);
  const to = doc.nodes.find((n) => n.id === edge.to);
  const wiring = makeWiring(registry);
  const others = doc.edges.filter((e) => e.id !== edge.id);
  const check = checkWire(wiring, doc.nodes, others, edge.from, edge.to);
  const options: Protocol[] = check.ok ? check.protocols : [edge.protocol];
  const name = (n?: Node) => (n ? n.label.trim() || flavourName(t, n.flavour) : "?");
  return (
    <div className="space-y-4" data-testid="inspector-edge">
      <div className="font-semibold">
        {name(from)} → {name(to)}
      </div>
      <Field label={t("inspector.protocol")} htmlFor={id}>
        <Select
          value={edge.protocol}
          onValueChange={(p) => setEdgeProtocol(edge.id, p as Protocol)}
        >
          <SelectTrigger id={id} className="w-full bg-card">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {options.map((p) => (
              <SelectItem key={p} value={p}>
                {t(`protocol.${p}`)}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </Field>
      <DeleteButton ids={[edge.id]} />
    </div>
  );
}

function DesignSummary({ registry }: { registry: Registry }) {
  const { t } = useTranslation();
  const doc = useEditor((s) => s.doc);
  const path = useEditor((s) => s.path);
  const rename = useEditor((s) => s.rename);
  return (
    <div className="space-y-4" data-testid="inspector-design">
      <TextField label={t("inspector.designName")} value={doc.name} max={registry.limits.max_label} onCommit={rename} />
      <dl className="grid grid-cols-2 gap-x-3 gap-y-1 text-sm">
        <dt className="text-muted-foreground">{t("inspector.count.nodes")}</dt>
        <dd className="text-right font-mono">{doc.nodes.length}</dd>
        <dt className="text-muted-foreground">{t("inspector.count.edges")}</dt>
        <dd className="text-right font-mono">{doc.edges.length}</dd>
        <dt className="text-muted-foreground">{t("inspector.count.zones")}</dt>
        <dd className="text-right font-mono">{doc.groups.length}</dd>
        <dt className="text-muted-foreground">{t("inspector.count.notes")}</dt>
        <dd className="text-right font-mono">{doc.notes.length}</dd>
      </dl>
      <p className="break-all text-xs text-muted-foreground">{path ?? t("inspector.notSaved")}</p>
      <p className="text-xs text-muted-foreground">{t("inspector.nothingSelected")}</p>
    </div>
  );
}

export function InspectorPanel({ registry }: { registry: Registry }) {
  const { t } = useTranslation();
  const doc = useEditor((s) => s.doc);
  const selection = useEditor((s) => s.selection);
  const problem = useProblemMode();
  const [tab, setTab] = useState<"brief" | "inspect" | "metrics">(problem ? "brief" : "inspect");
  // A run that starts from idle brings its metrics forward; selecting a card brings the
  // inspector back.
  useEffect(
    () =>
      useRun.subscribe((s, prev) => {
        if (s.status === "starting" && (prev.status === "idle" || prev.status === "finished")) setTab("metrics");
      }),
    [],
  );
  useEffect(() => {
    if (selection.length > 0) setTab("inspect");
  }, [selection]);
  let body: React.ReactNode;
  if (selection.length > 1) {
    body = (
      <div className="space-y-4" data-testid="inspector-multi">
        <p className="text-sm">{t("inspector.many", { count: selection.length })}</p>
        <DeleteButton ids={selection} />
      </div>
    );
  } else if (selection.length === 1) {
    const id = selection[0];
    const node = doc.nodes.find((n) => n.id === id);
    const group = doc.groups.find((g) => g.id === id);
    const note = doc.notes.find((n) => n.id === id);
    const edge = doc.edges.find((e) => e.id === id);
    body = node ? (
      <NodeInspector key={node.id} registry={registry} node={node} />
    ) : group ? (
      <GroupInspector key={group.id} registry={registry} group={group} />
    ) : note ? (
      <NoteInspector key={note.id} registry={registry} note={note} />
    ) : edge ? (
      <EdgeInspector key={edge.id} registry={registry} edge={edge} />
    ) : (
      <DesignSummary registry={registry} />
    );
  } else {
    body = <DesignSummary registry={registry} />;
  }
  const tabClass = (on: boolean) =>
    `-mb-px border-b-2 pb-2 ${on ? "border-primary font-medium" : "border-transparent text-muted-foreground hover:text-foreground"}`;
  return (
    <aside aria-label={t("inspector.title")} className="flex h-full min-h-0 flex-col bg-background" data-testid="inspector">
      <div className="flex gap-4 border-b px-4 pt-3 text-sm" role="tablist">
        {problem && (
          <button type="button" role="tab" id="tab-brief" aria-selected={tab === "brief"} aria-controls="tabpanel-side" className={tabClass(tab === "brief")} onClick={() => setTab("brief")}>
            {t("inspector.brief")}
          </button>
        )}
        <button type="button" role="tab" id="tab-inspect" aria-selected={tab === "inspect"} aria-controls="tabpanel-side" className={tabClass(tab === "inspect")} onClick={() => setTab("inspect")}>
          {t("inspector.title")}
        </button>
        <button type="button" role="tab" id="tab-metrics" aria-selected={tab === "metrics"} aria-controls="tabpanel-side" className={tabClass(tab === "metrics")} onClick={() => setTab("metrics")}>
          {t("inspector.metrics")}
        </button>
      </div>
      <div className="min-h-0 flex-1 overflow-y-auto p-4" role="tabpanel" id="tabpanel-side" aria-labelledby={`tab-${tab}`}>
        {tab === "brief" && problem ? <BriefPanel brief={problem.brief} /> : tab === "metrics" ? <MetricsPanel /> : body}
      </div>
    </aside>
  );
}
