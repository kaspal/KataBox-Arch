// Checks that need no rendering: i18n key parity and coverage, error codes, the registry
// fixture rule, and WCAG contrast of every token pair.
import { describe, expect, it } from "vitest";
import i18next from "i18next";
import { readdirSync, readFileSync, statSync } from "node:fs";
import path from "node:path";
import type { AppErrorCode } from "@/bindings";
import en from "@/i18n/locales/en.json";
import es from "@/i18n/locales/es.json";
import { resources } from "@/i18n";
import { describeError } from "@/lib/errors";
import fixture from "@/test/fixtures/registry.json";
import { SCHEMA_VERSION } from "@/solve/store";

function keys(obj: object, prefix = ""): string[] {
  return Object.entries(obj).flatMap(([k, v]) =>
    typeof v === "object" && v !== null ? keys(v, `${prefix}${k}.`) : [`${prefix}${k}`],
  );
}

function has(res: object, key: string): boolean {
  const v = key.split(".").reduce<unknown>((o, p) => (o as Record<string, unknown> | undefined)?.[p], res);
  return typeof v === "string" && v.trim() !== "";
}

describe("i18n resources", () => {
  it("English and Spanish have the same keys", () => {
    expect(keys(es).sort()).toEqual(keys(en).sort());
  });

  it("no translation is empty", () => {
    for (const [lang, res] of [["en", en], ["es", es]] as const) {
      expect(keys(res).filter((k) => !has(res, k)), lang).toEqual([]);
    }
  });

  it("every error code has a message", () => {
    // `satisfies` makes this list fail to compile when Rust adds a code.
    const codes = {
      not_found: true,
      invalid_input: true,
      store: true,
      io: true,
      file_too_large: true,
      invalid_file: true,
      newer_file_schema: true,
      invalid_design: true,
      setting_not_allowed: true,
      path_not_allowed: true,
      internal: true,
      no_path: true,
    } satisfies Record<AppErrorCode, true>;
    expect(Object.keys(codes).filter((c) => !has(en, `errors.${c}`))).toEqual([]);
  });

  it("every registry id the UI shows has a name (kinds, flavours, props, choices, units, reasons)", () => {
    const r = fixture.registry;
    const wanted = [
      ...r.categories.map((c) => `registry.category.${c}`),
      ...r.kinds.map((k) => `registry.kind.${k.kind}`),
      ...r.kinds.flatMap((k) => k.flavours.map((f) => `registry.flavour.${f.flavour}`)),
      ...r.kinds.flatMap((k) => k.props.map((p) => `registry.prop.${p.key}`)),
      ...r.kinds.flatMap((k) =>
        k.props.flatMap((p) => ("options" in p.ty ? (p.ty.options as string[]).map((o) => `registry.choice.${o}`) : [])),
      ),
      ...r.kinds.flatMap((k) => k.props.map((p) => `units.${p.unit}`)),
      ...r.wiring.denials.map((d) => `wiring.reason.${d.reason}`),
      ...["self_loop", "duplicate_edge", "no_rule"].map((c) => `wiring.reason.${c}`),
      ...r.wiring.rules.map((x) => `protocol.${x.protocol}`),
    ];
    expect([...new Set(wanted)].filter((k) => !has(en, k))).toEqual([]);
  });
});

describe("error codes", () => {
  const i18n = i18next.createInstance();
  void i18n.init({ resources, lng: "es", fallbackLng: "en", initAsync: false });

  it("maps known codes to the UI language and keeps the detail", () => {
    expect(describeError(i18n, { code: "file_too_large", message: "the file is larger than 8 MiB" })).toEqual({
      summary: "El archivo es demasiado grande para abrirlo.",
      detail: "the file is larger than 8 MiB",
    });
  });

  it("falls back to the English text for unknown codes, and to generic for anything else", () => {
    expect(describeError(i18n, { code: "brand_new", message: "Something new" })).toEqual({ summary: "Something new" });
    expect(describeError(i18n, new Error("boom"))).toEqual({ summary: "Algo salió mal.", detail: "boom" });
  });
});

describe("source rules", () => {
  const src = path.resolve(process.cwd(), "src");
  const files = (dir: string): string[] =>
    readdirSync(dir).flatMap((f) => {
      const p = path.join(dir, f);
      return statSync(p).isDirectory() ? files(p) : [p];
    });
  const appCode = files(src).filter(
    (f) => /\.(ts|tsx)$/.test(f) && !/\.test\.tsx?$/.test(f) && !f.includes(`${path.sep}test${path.sep}`),
  );

  it("the editor writes the .kbarch version Rust reads and writes", () => {
    expect(SCHEMA_VERSION).toBe(fixture.schema_version);
  });

  it("app code never imports test fixtures (the registry comes from Rust)", () => {
    expect(appCode.filter((f) => readFileSync(f, "utf8").includes("test/fixtures"))).toEqual([]);
  });

  it("app code never imports the registry fixture (the registry comes from Rust)", () => {
    expect(appCode.filter((f) => readFileSync(f, "utf8").includes("fixtures/registry"))).toEqual([]);
  });

  it("only lib/api.ts imports the generated commands", () => {
    const bad = appCode.filter((f) => {
      const text = readFileSync(f, "utf8");
      return /import\s*\{[^}]*\bcommands\b[^}]*\}\s*from\s*"@\/bindings"/.test(text) && !f.endsWith(path.join("lib", "api.ts"));
    });
    expect(bad).toEqual([]);
  });

  it("shadcn components import cn from our utils, never the npm package \"cn\"", () => {
    const dir = path.join(src, "components/ui");
    expect(readdirSync(dir).filter((f) => readFileSync(path.join(dir, f), "utf8").includes('from "cn"'))).toEqual([]);
  });
});

// --- WCAG contrast of the design tokens -----------------------------------------------

// Vitest runs from apps/desktop.
const tokensCss = readFileSync(path.resolve(process.cwd(), "src/styles/tokens.css"), "utf8");

function block(selector: string): Record<string, string> {
  const start = tokensCss.indexOf(`\n${selector} {`);
  expect(start, selector).toBeGreaterThanOrEqual(0);
  const body = tokensCss.slice(start, tokensCss.indexOf("\n}", start));
  return Object.fromEntries([...body.matchAll(/--([\w-]+):\s*(#[0-9a-fA-F]{6})/g)].map((m) => [m[1], m[2]]));
}

function rgb(hex: string): number[] {
  return [1, 3, 5].map((i) => parseInt(hex.slice(i, i + 2), 16));
}

function luminance(hex: string): number {
  const [r, g, b] = rgb(hex).map((v) => {
    const c = v / 255;
    return c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
  });
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}

function contrast(a: string, b: string): number {
  const [x, y] = [luminance(a), luminance(b)].sort((p, q) => q - p);
  return (x + 0.05) / (y + 0.05);
}

/** `color-mix(in srgb, fg p, bg)`, as the icon tiles do. */
function mix(fg: string, bg: string, p: number): string {
  const [f, b] = [rgb(fg), rgb(bg)];
  return `#${f.map((v, i) => Math.round(v * p + b[i] * (1 - p)).toString(16).padStart(2, "0")).join("")}`;
}

/** Surfaces text sits on. The desk is a frame only: no text on it. */
const SURFACES = ["background", "card", "popover", "canvas", "muted", "accent"];
/** Colours used as text. Terracotta, amber and yellow are fills only (their -ink is text). */
const TEXT = [
  "foreground",
  "card-foreground",
  "popover-foreground",
  "accent-foreground",
  "muted-foreground",
  "primary",
  "primary-ink",
  "terracotta-ink",
  "green",
  "amber-ink",
  "destructive",
];
const CATEGORIES = ["edge", "compute", "data", "async", "ops"];

const LIGHT = block(":root");
const DARK = { ...LIGHT, ...block(".dark") };
const LIGHT_HC = { ...LIGHT, ...block(".hc") };
const DARK_HC = { ...DARK, ...block(".dark.hc") };

describe.each([
  ["light", LIGHT],
  ["dark", DARK],
  ["light high contrast", LIGHT_HC],
  ["dark high contrast", DARK_HC],
])("%s theme contrast", (name, t) => {
  const hc = name.includes("high");

  it.each(TEXT)("%s text is ≥ 4.5:1 on every surface (WCAG AA)", (fg) => {
    for (const bg of SURFACES) {
      expect(contrast(t[fg], t[bg]), `${fg} on ${bg}`).toBeGreaterThanOrEqual(4.5);
    }
  });

  it("filled colours keep their text readable", () => {
    const min = hc ? 7 : 4.5;
    expect(contrast(t["primary-foreground"], t.primary)).toBeGreaterThanOrEqual(min);
    expect(contrast(t["destructive-foreground"], t.destructive)).toBeGreaterThanOrEqual(4.5);
    expect(contrast(t["dev-badge-foreground"], t["dev-badge"])).toBeGreaterThanOrEqual(4.5);
    // Tooltips: background-coloured text on a foreground fill.
    expect(contrast(t.background, t.foreground)).toBeGreaterThanOrEqual(4.5);
    // Sticky notes: the note's text and its placeholder.
    expect(contrast(t["note-foreground"], t.note)).toBeGreaterThanOrEqual(4.5);
    expect(contrast(t["muted-foreground"], t.note)).toBeGreaterThanOrEqual(4.5);
  });

  it.each(CATEGORIES)("the %s glyph stands out on its 14%% tile (≥ 3:1, WCAG 1.4.11)", (c) => {
    const tile = mix(t[`cat-${c}`], t.card, 0.14);
    expect(contrast(t[`cat-${c}-glyph`], tile)).toBeGreaterThanOrEqual(3);
  });

  it("the focus ring, edges and zone frames stand out (≥ 3:1)", () => {
    for (const bg of ["background", "card"]) {
      expect(contrast(t.ring, t[bg]), `ring on ${bg}`).toBeGreaterThanOrEqual(3);
    }
    expect(contrast(t["edge-line"], t.canvas), "edge-line on canvas").toBeGreaterThanOrEqual(3);
    expect(contrast(t["zone-line"], t.canvas), "zone-line on canvas").toBeGreaterThanOrEqual(3);
  });

  if (hc) {
    it.each(["foreground", "muted-foreground", "primary", "card-foreground", "popover-foreground", "accent-foreground"])(
      "%s text reaches 7:1 on every surface (WCAG AAA)",
      (fg) => {
        for (const bg of SURFACES) {
          expect(contrast(t[fg], t[bg]), `${fg} on ${bg}`).toBeGreaterThanOrEqual(7);
        }
      },
    );
    it("borders and inputs are ≥ 3:1", () => {
      for (const line of ["border", "input"]) {
        for (const bg of ["background", "card"]) {
          expect(contrast(t[line], t[bg]), `${line} on ${bg}`).toBeGreaterThanOrEqual(3);
        }
      }
    });
  }
});
