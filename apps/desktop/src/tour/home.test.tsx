import { describe, expect, it, vi } from "vitest";
import { act, fireEvent, screen, waitFor, within } from "@testing-library/react";
import { setFakeApi, useFakeApi } from "@/test/mockApi";
import { renderApp } from "@/test/renderApp";
import { ONE } from "@/test/fakeSamples";
import { isActive, useRun } from "@/solve/run/runStore";

vi.mock("@/lib/api", async () => (await import("@/test/mockApi")).apiModule);

const fake = useFakeApi();

describe("Home's samples", () => {
  it("shows the gallery; the card under the pointer plays live, the others stay still", async () => {
    renderApp("/");
    const card = await screen.findByTestId("home-sample");
    expect(card).toHaveTextContent("Hello, Traffic");
    expect(within(screen.getByTestId("home-samples")).getByRole("link", { name: "All samples" })).toBeInTheDocument();
    // No IntersectionObserver in the test DOM: nothing plays until pointed at.
    expect(screen.queryByTestId("live-preview")).toBeNull();
    expect(fake().runs).toBe(0);

    fireEvent.mouseEnter(card.closest("li")!);
    expect(await screen.findByTestId("live-preview")).toBeInTheDocument();
    await waitFor(() => expect(fake().runs).toBe(1));
    const [design, opts] = vi.mocked(fake().api.startRun).mock.calls[0];
    expect(design.id).toBe(ONE.id);
    expect(opts).toMatchObject({ preset: "spike", startS: 18 });

    fireEvent.mouseLeave(card.closest("li")!);
    await waitFor(() => expect(screen.queryByTestId("live-preview")).toBeNull());
    expect(isActive(useRun.getState().status)).toBe(false);
    expect(useRun.getState()).toMatchObject({ preset: "normal", problemTest: null });
  });

  it("the live preview loops every 6 seconds", async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    try {
      renderApp("/");
      const card = await screen.findByTestId("home-sample");
      fireEvent.focus(card);
      await waitFor(() => expect(fake().runs).toBe(1));
      await act(async () => {
        vi.advanceTimersByTime(6000);
      });
      await waitFor(() => expect(fake().runs).toBe(2));
      fireEvent.blur(card);
      await waitFor(() => expect(screen.queryByTestId("live-preview")).toBeNull());
    } finally {
      vi.useRealTimers();
    }
  });

  it("with reduced motion, no preview plays", async () => {
    setFakeApi({ settings: { "ui.motion": "reduced" } });
    document.documentElement.classList.add("reduce-motion");
    try {
      renderApp("/");
      const card = await screen.findByTestId("home-sample");
      fireEvent.mouseEnter(card.closest("li")!);
      await new Promise((r) => setTimeout(r, 50));
      expect(screen.queryByTestId("live-preview")).toBeNull();
      expect(fake().runs).toBe(0);
    } finally {
      document.documentElement.classList.remove("reduce-motion");
    }
  });
});

describe("the first-run tour offer", () => {
  it("is offered once: Start opens the tour and remembers", async () => {
    const { user, history } = renderApp("/");
    const offer = await screen.findByTestId("tour-offer");
    expect(offer).toHaveTextContent("New here? Take the tour");
    await user.click(within(offer).getByRole("button", { name: "Start the tour" }));
    await waitFor(() => expect(history.location.pathname).toBe("/samples/hello-traffic"));
    expect(fake().api.setSetting).toHaveBeenCalledWith("tour.offered", true);
  });

  it("Not now hides it for good", async () => {
    const { user } = renderApp("/");
    const offer = await screen.findByTestId("tour-offer");
    await user.click(within(offer).getByRole("button", { name: "Not now" }));
    await waitFor(() => expect(screen.queryByTestId("tour-offer")).toBeNull());
    expect(fake().api.setSetting).toHaveBeenCalledWith("tour.offered", true);
  });

  it("isn't offered again", async () => {
    setFakeApi({ settings: { "tour.offered": true } });
    renderApp("/");
    await screen.findByTestId("home-sample");
    expect(screen.queryByTestId("tour-offer")).toBeNull();
  });
});
