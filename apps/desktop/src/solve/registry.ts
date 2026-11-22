// Helpers over the registry Rust sends (no component data is defined here).
import type { TFunction } from "i18next";
import type { ComponentKind, Flavour, KindSpec, PropValue, Registry } from "@/bindings";

export function kindSpec(registry: Registry, kind: ComponentKind): KindSpec {
  const spec = registry.kinds.find((k) => k.kind === kind);
  if (!spec) throw new Error(`unknown kind ${kind}`);
  return spec;
}

/** The props a new card of `kind` / `flavour` starts with: every prop, at the kind's
 * default with the flavour's overrides applied. */
export function defaultProps(spec: KindSpec, flavour: Flavour): Partial<{ [key in string]: PropValue }> {
  const props: { [key in string]: PropValue } = {};
  for (const p of spec.props) props[p.key] = p.default;
  for (const o of spec.flavours.find((f) => f.flavour === flavour)?.overrides ?? []) props[o.key] = o.value;
  return props;
}

// Display names live in i18n, keyed by the registry's ids.
export const kindName = (t: TFunction, kind: ComponentKind) => t(`registry.kind.${kind}` as "registry.kind.client");
export const flavourName = (t: TFunction, f: Flavour) => t(`registry.flavour.${f}` as "registry.flavour.web");
export const categoryName = (t: TFunction, c: string) => t(`registry.category.${c}` as "registry.category.edge");
export const propName = (t: TFunction, key: string) => t(`registry.prop.${key}` as "registry.prop.instances");
export const unitName = (t: TFunction, unit: string) => t(`units.${unit}` as "units.ms");
export const choiceName = (t: TFunction, option: string) => t(`registry.choice.${option}` as "registry.choice.lru");
