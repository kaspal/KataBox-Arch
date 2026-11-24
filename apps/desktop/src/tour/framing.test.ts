// Every sample and concept-card step shows all its focused cards (M1–M3 review F2): at two
// window sizes, each focused card (or every card, for a step without focus) lies inside the
// view above the caption band, with the margin. The fixture comes from the content
// (`cargo test -p ka-pack --test stories` keeps it in step).
import { describe, expect, it } from "vitest";
import type { Design, StoryStep } from "@/bindings";
import { CARD_H, CARD_W } from "@/solve/store";
import { designBounds, fitBox } from "@/solve/compare/DesignView";
import { FRAME_MARGIN, MIN_READABLE_ZOOM, stepViewport, visibleArea } from "@/tour/Player";
import stories from "@/test/fixtures/stories.json";

type FixtureStep = { focus: string[]; camera: { x: number; y: number; zoom: number } | null; callout: boolean; nodes: { id: string; x: number; y: number }[] };
type FixtureStory = { kind: string; id: string; steps: FixtureStep[] };

const SIZES = [
  { w: 960, h: 540 },
  { w: 1280, h: 720 },
];

function design(step: FixtureStep): Design {
  return {
    schema_version: 3,
    id: "d",
    name: "d",
    problem_id: null,
    nodes: step.nodes.map((n) => ({ id: n.id, kind: "app", flavour: "go", label: "", position: { x: n.x, y: n.y }, props: {}, group_id: null })),
    edges: [],
    groups: [],
    notes: [],
    viewport: { x: 0, y: 0, zoom: 1 },
  };
}

describe("story framing", () => {
  const all = stories as FixtureStory[];
  it("covers the shipped stories", () => {
    expect(all.filter((s) => s.kind === "sample").length).toBeGreaterThanOrEqual(5);
    expect(all.some((s) => s.id === "hello-traffic")).toBe(true);
  });

  for (const story of all) {
    story.steps.forEach((step, i) => {
      it(`${story.kind} ${story.id} step ${i + 1} shows every focused card`, () => {
        const d = design(step);
        const lit = step.focus.length > 0 ? step.nodes.filter((n) => step.focus.includes(n.id)) : step.nodes;
        for (const size of SIZES) {
          const v = stepViewport({ focus: step.focus, camera: step.camera ?? undefined, callout: step.callout ? {} : undefined } as StoryStep, d, size);
          // The canvas nothing covers: above the step card, left of the metric tile (F6).
          const area = visibleArea(size, step.callout);
          // H6: the zoom stays readable; a focus too wide for it is centred instead.
          expect(v.zoom, `${story.id} step ${i + 1} zoom`).toBeGreaterThanOrEqual(MIN_READABLE_ZOOM - 1e-9);
          const xs = lit.map((n) => n.x);
          const ys = lit.map((n) => n.y);
          const focusFit = fitBox({ x: Math.min(...xs), y: Math.min(...ys), w: Math.max(...xs) + CARD_W - Math.min(...xs), h: Math.max(...ys) + CARD_H - Math.min(...ys) }, area, FRAME_MARGIN, 0.05).zoom;
          if (focusFit < MIN_READABLE_ZOOM) continue;
          for (const n of lit) {
            const x0 = n.x * v.zoom + v.x;
            const y0 = n.y * v.zoom + v.y;
            const x1 = (n.x + CARD_W) * v.zoom + v.x;
            const y1 = (n.y + CARD_H) * v.zoom + v.y;
            const where = `${n.id} at ${size.w}×${size.h}: [${x0.toFixed(0)}, ${y0.toFixed(0)}]–[${x1.toFixed(0)}, ${y1.toFixed(0)}]`;
            expect(x0, where).toBeGreaterThanOrEqual(FRAME_MARGIN - 0.5);
            expect(y0, where).toBeGreaterThanOrEqual(FRAME_MARGIN - 0.5);
            expect(x1, where).toBeLessThanOrEqual(area.w - FRAME_MARGIN + 0.5);
            expect(y1, where).toBeLessThanOrEqual(area.h - FRAME_MARGIN + 0.5);
          }
          // No card, focused or not, is cut off or hard against the visible edge: each is fully
          // out of view, or in it with the whole margin. H6: except when showing every card
          // would shrink them below the readable zoom; then the zoom stays readable.
          const whole = fitBox(designBounds(d), area, FRAME_MARGIN, 0.05).zoom;
          const tooWide = whole < MIN_READABLE_ZOOM;
          for (const n of step.nodes.filter(() => !tooWide)) {
            const x0 = n.x * v.zoom + v.x;
            const y0 = n.y * v.zoom + v.y;
            const x1 = (n.x + CARD_W) * v.zoom + v.x;
            const y1 = (n.y + CARD_H) * v.zoom + v.y;
            const m = FRAME_MARGIN - 0.5;
            const inside = x0 >= m && y0 >= m && x1 <= area.w - m && y1 <= area.h - m;
            const outside = x1 <= 0 || y1 <= 0 || x0 >= area.w || y0 >= area.h;
            expect(inside || outside, `${n.id} is cut off or too close to the edge at ${size.w}×${size.h}: [${x0.toFixed(0)}, ${y0.toFixed(0)}]`).toBe(true);
          }
        }
      });
    });
  }
});
