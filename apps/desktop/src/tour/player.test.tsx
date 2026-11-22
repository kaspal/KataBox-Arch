import { describe, expect, it, vi } from "vitest";
import { act, screen, waitFor, within } from "@testing-library/react";
import { useFakeApi } from "@/test/mockApi";
import { renderApp } from "@/test/renderApp";
import { unnamed } from "@/test/a11y";
import { FAKE_SAMPLE, ONE } from "@/test/fakeSamples";
import { CARD_H, CARD_W, useEditor } from "@/solve/store";
import { isActive, useRun } from "@/solve/run/runStore";
import { stepViewport, visibleArea } from "@/tour/Player";

vi.mock("@/lib/api", async () => (await import("@/test/mockApi")).apiModule);

const fake = useFakeApi();

describe("the sample gallery and the player", () => {
  it("lists samples and plays one step by step, with traffic, captions and callouts", async () => {
    const { user, history } = renderApp("/samples");
    const card = await screen.findByTestId("sample-card");
    expect(card).toHaveTextContent("Hello, Traffic");
    expect(card).toHaveTextContent("2 steps");
    expect(card).toHaveTextContent("The first-run tour");
    await user.click(card);
    await waitFor(() => expect(history.location.pathname).toBe("/samples/hello-traffic"));

    const player = await screen.findByTestId("player");
    expect(within(player).getByRole("heading", { name: "Hello, Traffic" })).toBeInTheDocument();
    expect(screen.getByTestId("step-counter")).toHaveTextContent("step 1 of 2");
    expect(screen.getByTestId("caption")).toHaveTextContent("Step 1 · One server");
    expect(screen.getByTestId("caption")).toHaveTextContent("Requests leave the client and reach the server.");
    expect(screen.getByTestId("callout")).toHaveTextContent("p95 latency");
    expect(screen.getByTestId("callout")).toHaveTextContent("420 ms");
    // Step 1 plays the Sandbox spike from 18 s on its design.
    await waitFor(() => expect(fake().runs).toBe(1));
    const [design1, opts1] = vi.mocked(fake().api.startRun).mock.calls[0];
    expect(design1.id).toBe("hello-one");
    expect(opts1).toMatchObject({ preset: "spike", problem: null, startS: 18, speed: 1 });
    // The spotlight: the client dims, the focused server doesn't.
    const cards = within(screen.getByTestId("player-canvas")).getAllByTestId("component-card");
    expect(cards).toHaveLength(2);
    expect(cards[0].closest(".react-flow__node")).toHaveClass("kb-dim");
    expect(cards[1].closest(".react-flow__node")).not.toHaveClass("kb-dim");

    await user.click(screen.getByRole("button", { name: "Next: Share the load" }));
    expect(screen.getByTestId("step-counter")).toHaveTextContent("step 2 of 2");
    expect(screen.getByTestId("callout")).toHaveTextContent("18 ms");
    expect(within(screen.getByTestId("callout")).getByLabelText("Before: 420 ms")).toBeInTheDocument();
    await waitFor(() => expect(fake().runs).toBe(2));
    const [design2, opts2] = vi.mocked(fake().api.startRun).mock.calls[1];
    expect(design2.id).toBe("hello-two");
    expect(opts2).toMatchObject({ problem: { problem_id: "url-shortener", test_id: "normal" }, startS: 0, speed: 2 });
    expect(screen.getByRole("button", { name: "The end" })).toBeDisabled();

    // Back with the dots or the keyboard; Replay starts the step's run again.
    await user.click(screen.getByRole("button", { name: "Step 1: One server" }));
    expect(screen.getByTestId("step-counter")).toHaveTextContent("step 1 of 2");
    await user.keyboard("{ArrowRight}");
    expect(screen.getByTestId("step-counter")).toHaveTextContent("step 2 of 2");
    const before = fake().runs;
    await user.click(screen.getByRole("button", { name: "Replay step" }));
    await waitFor(() => expect(fake().runs).toBe(before + 1));
    expect(unnamed(player)).toEqual([]);

    // Leaving the player stops its run and gives back the run settings it changed.
    act(() => void history.push("/"));
    await waitFor(() => expect(isActive(useRun.getState().status)).toBe(false));
    expect(useRun.getState()).toMatchObject({ preset: "normal", problemTest: null, speed: 1 });
  });

  it("Try it yourself copies the step's design into the Sandbox", async () => {
    const { user, history } = renderApp("/samples/hello-traffic");
    await screen.findByTestId("player");
    await user.click(screen.getByRole("button", { name: "Try it yourself" }));
    await waitFor(() => expect(history.location.pathname).toBe("/sandbox"));
    const doc = useEditor.getState().doc;
    expect(doc.name).toBe("Hello, Traffic (my copy)");
    expect(doc.problem_id).toBeNull();
    expect(doc.nodes).toHaveLength(ONE.nodes.length);
    expect(doc.nodes.map((n) => n.id)).not.toContain("app");
    expect(doc.id).not.toBe(ONE.id);
  });

  it("an unknown sample says so", async () => {
    renderApp("/samples/nope");
    expect(await screen.findByText("Samples are unavailable")).toBeInTheDocument();
  });
});

describe("the camera", () => {
  const size = { w: 1000, h: 800 };
  it("centres a step's own camera point above the caption band", () => {
    const step = FAKE_SAMPLE.def.steps[1];
    // 800 tall less the 160 px caption band: the point sits at y = 120. The step has a metric
    // tile, so it is centred in the 744 px left of the tile's column (F6): x = 272.
    expect(step.callout).toBeTruthy();
    expect(stepViewport(step, FAKE_SAMPLE.snapshots["two.kbarch"]!, size)).toEqual({ x: 272, y: 120, zoom: 1 });
  });
  it("a step's own camera never cuts off a focused card: it zooms out and slides", () => {
    // The camera looks far to the right of the focused card, zoomed in.
    const step = { ...FAKE_SAMPLE.def.steps[0], camera: { x: 2000, y: 0, zoom: 2 } };
    const v = stepViewport(step, ONE, size);
    expect(v.zoom).toBeLessThanOrEqual(1.2);
    const card = ONE.nodes.find((n) => (step.focus ?? []).includes(n.id))!;
    const x0 = card.position.x * v.zoom + v.x;
    expect(x0).toBeGreaterThanOrEqual(48 - 0.5);
  });
  it("otherwise fits the focused cards (at most 100%), leaving room for the caption", () => {
    const step = FAKE_SAMPLE.def.steps[0];
    const v = stepViewport(step, ONE, size);
    expect(v.zoom).toBe(1);
    // The focused card (at 0,200) is centred in the visible canvas: above the caption band,
    // and left of the metric tile when the step has one (F6).
    const area = visibleArea(size, !!step.callout);
    expect((0 + CARD_W / 2) * v.zoom + v.x).toBe(area.w / 2);
    expect((200 + CARD_H / 2) * v.zoom + v.y).toBe(area.h / 2);
  });
});
