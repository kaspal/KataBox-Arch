// What a Guide "In the simulator" row shows for one flavour: the registry's own number (a
// prop default with the flavour's overrides, a hidden simulator default, or a price), so
// the Guide can never disagree with the simulator.
import type { TFunction } from "i18next";
import type { Flavour, KindSpec, SimRow } from "@/bindings";
import { choiceName, defaultProps, propName, unitName } from "@/solve/registry";

export function simLabel(t: TFunction, v: SimRow): string {
  if ("prop" in v) return propName(t, v.prop);
  if ("sim" in v) return t(`guide.simField.${v.sim}` as "guide.simField.service_ms");
  return t(`guide.costField.${v.cost}` as "guide.costField.per_million");
}

export function simValue(t: TFunction, lang: string, spec: KindSpec, flavour: Flavour, v: SimRow): string {
  const num = (n: number, digits = 2) => new Intl.NumberFormat(lang, { maximumFractionDigits: digits }).format(n);
  const f = spec.flavours.find((x) => x.flavour === flavour);
  if ("prop" in v) {
    const p = spec.props.find((x) => x.key === v.prop);
    const value = defaultProps(spec, flavour)[v.prop];
    if (!p || value === undefined) return "—";
    if (typeof value === "boolean") return value ? t("guide.yes") : t("guide.no");
    if (typeof value === "string") return choiceName(t, value);
    const unit = p.unit === "none" || p.unit === "count" ? "" : ` ${unitName(t, p.unit)}`;
    return `${num(value)}${unit}`;
  }
  if ("sim" in v) {
    const s = f?.sim as Record<string, number> | null | undefined;
    const n = s?.[v.sim];
    if (n === undefined) return "—";
    switch (v.sim) {
      case "service_ms":
        return `${num(n)} ${unitName(t, "ms")}`;
      case "failover_s":
        return `${num(n)} ${unitName(t, "s")}`;
      case "p99_ratio":
      case "write_slots":
        return `×${num(n)}`;
      default:
        return num(n);
    }
  }
  const c = f?.cost as Record<string, number | null> | undefined;
  const meters = f?.cost.meters as Record<string, number> | undefined;
  // A write price left unset bills writes as reads; usage meters live under `meters`.
  const n = v.cost === "per_million_write" ? (c?.per_million_write ?? c?.per_million) : (meters?.[v.cost] ?? c?.[v.cost]);
  if (n === undefined || n === null) return "—";
  const usd = new Intl.NumberFormat(lang, { style: "currency", currency: "USD", minimumFractionDigits: 0, maximumFractionDigits: 2 }).format(n);
  return t(`guide.costUnit.${v.cost}` as "guide.costUnit.per_million", { v: usd });
}
