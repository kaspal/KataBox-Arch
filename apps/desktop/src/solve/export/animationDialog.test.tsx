import { describe, expect, it, vi } from "vitest";
import { fireEvent, screen, waitFor, within } from "@testing-library/react";
import { setFakeApi, useFakeApi } from "@/test/mockApi";
import { renderApp } from "@/test/renderApp";
import { unnamed } from "@/test/a11y";
import { useRun } from "@/solve/run/runStore";

vi.mock("@/lib/api", async () => (await import("@/test/mockApi")).apiModule);
// jsdom has no 2D canvas or MediaRecorder: stand in for the recorder, keep the unpacking.
const recorded = vi.hoisted(() => ({ frames: 0 }));
vi.mock("@/solve/export/animation", async (orig) => {
  const real = await orig<typeof import("@/solve/export/animation")>();
  return {
    ...real,
    webmType: () => "video/webm",
    renderer: () => ({ canvas: {} as HTMLCanvasElement, paint: () => {} }),
    recordWebm: async (_r: unknown, n: number) => {
      recorded.frames = n;
      return new Uint8Array([26, 69, 223, 163]);
    },
  };
});
vi.mock("@/solve/export/scene", async (orig) => ({
  ...(await orig<typeof import("@/solve/export/scene")>()),
  svgImage: async () => ({}) as HTMLImageElement,
}));

const fake = useFakeApi();

describe("saving an animation (M7b.2)", () => {
  it("records the Sandbox run from the Export menu and saves it through Rust", async () => {
    const { user } = renderApp("/sandbox");
    useRun.setState({ preset: "spike" });
    await user.click(await screen.findByTestId("export-menu"));
    await user.click(await screen.findByRole("menuitem", { name: "Save animation (WebM)…" }));
    const dialog = await screen.findByTestId("animation-dialog");
    expect(dialog).toHaveTextContent("Sandbox · Spike ×4");
    expect(dialog).toHaveTextContent("records in real time");
    expect(unnamed(dialog)).toEqual([]);
    const length = within(dialog).getByLabelText("Length (s, at most 60)");
    fireEvent.change(length, { target: { value: "5" } });
    await user.click(within(dialog).getByRole("button", { name: "Record" }));
    await waitFor(() => expect(fake().api.saveExport).toHaveBeenCalled());
    const [req] = vi.mocked(fake().api.runFrames).mock.calls[0]!;
    expect(req).toMatchObject({ source: { type: "sandbox", preset: "spike" }, from_s: 0, seconds: 5, fps: 30 });
    expect(recorded.frames).toBe(150);
    const [format, bytes, name] = vi.mocked(fake().api.saveExport).mock.calls[0]!;
    expect(format).toBe("webm");
    expect(Array.from(bytes)).toEqual([26, 69, 223, 163]);
    expect(name).toMatch(/Sandbox · Spike ×4\.webm$/);
  });

  it("exports a failed test's replay from Results, from where Replay starts", async () => {
    setFakeApi({ verdict: "slo_breach" });
    const { user } = renderApp("/problems/url-shortener");
    await screen.findByTestId("brief");
    await user.click(screen.getByRole("button", { name: "Submit" }));
    const results = await screen.findByTestId("results");
    const rows = within(results).getAllByTestId("test-result");
    await user.click(within(rows[1]!).getByRole("button", { name: "Export replay" }));
    const dialog = await screen.findByTestId("animation-dialog");
    expect(dialog).toHaveTextContent("Link goes viral");
    expect(within(dialog).getByLabelText("Start (s)")).toHaveValue(20);
    await user.click(within(dialog).getByRole("button", { name: "Record" }));
    await waitFor(() => expect(fake().api.saveExport).toHaveBeenCalled());
    const [req] = vi.mocked(fake().api.runFrames).mock.calls[0]!;
    expect(req.source).toEqual({ type: "submission", submission_id: expect.any(Number), test_id: "viral" });
    expect(req.from_s).toBe(20);
  });
});
