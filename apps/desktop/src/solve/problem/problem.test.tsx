import { describe, expect, it, vi } from "vitest";
import { act, screen, waitFor, within } from "@testing-library/react";
import { setFakeApi, useFakeApi } from "@/test/mockApi";
import { renderApp } from "@/test/renderApp";
import { unnamed } from "@/test/a11y";
import { useEditor } from "@/solve/store";
import { useRun } from "@/solve/run/runStore";

vi.mock("@/lib/api", async () => (await import("@/test/mockApi")).apiModule);
const fake = useFakeApi();

describe("the problem catalogue", () => {
  it("lists problems with difficulty and status, and filters them", async () => {
    const { user } = renderApp("/problems");
    const row = await screen.findByTestId("problem-row");
    expect(row).toHaveTextContent("#1");
    expect(row).toHaveTextContent("URL Shortener");
    expect(row).toHaveTextContent("Easy");
    expect(row).toHaveTextContent("Not started");
    await user.type(screen.getByRole("searchbox", { name: "Search problems" }), "cache");
    expect(screen.getAllByTestId("problem-row")).toHaveLength(1);
    await user.click(screen.getByRole("button", { name: "Hard" }));
    expect(screen.queryByTestId("problem-row")).not.toBeInTheDocument();
    expect(screen.getByText("No problem matches.")).toBeInTheDocument();
    expect(unnamed(document.body)).toEqual([]);
  });

  it("Home offers the recommended next problem", async () => {
    renderApp("/");
    expect(await screen.findByTestId("home-recommended")).toHaveTextContent("URL Shortener");
  });
});

describe("solving a problem", () => {
  it("the brief shows the statement (no raw HTML), targets, hidden tests by name and hints on demand", async () => {
    const { user } = renderApp("/problems/url-shortener");
    const brief = await screen.findByTestId("brief");
    expect(within(brief).getByRole("heading", { name: "URL Shortener" })).toBeInTheDocument();
    // GitHub tables render; raw HTML in content is dropped.
    expect(within(brief).getByRole("table")).toBeInTheDocument();
    expect(brief.querySelector("b")).toBeNull();
    expect(brief).toHaveTextContent("12,000 req/s");
    expect(brief).toHaveTextContent("≤ 80 ms");
    const tests = within(brief).getByTestId("tests");
    expect(tests).toHaveTextContent("Link goes viral");
    expect(tests).toHaveTextContent("hidden");
    await user.click(within(brief).getByRole("button", { name: "Show hint 1" }));
    expect(await within(brief).findByText("Look at the reads.")).toBeInTheDocument();
    expect(fake().api.unlockHint).toHaveBeenCalledWith("url-shortener", 1);
    expect(within(brief).getByRole("button", { name: "Show hint 2" })).toBeInTheDocument();
    // The design belongs to the problem.
    expect(useEditor.getState().doc.problem_id).toBe("url-shortener");
  });

  it("Submit judges the design; a failing hidden test names its bottleneck and replays from it", async () => {
    setFakeApi({ verdict: "slo_breach" });
    const { user } = renderApp("/problems/url-shortener");
    await screen.findByTestId("brief");
    await user.click(screen.getByRole("button", { name: "Submit" }));
    const results = await screen.findByTestId("results");
    expect(fake().api.submitDesign).toHaveBeenCalledWith("url-shortener", expect.objectContaining({ problem_id: "url-shortener" }), expect.stringMatching(/^\d{4}-\d{2}-\d{2}$/));
    expect(within(results).getAllByTestId("verdict")[0]).toHaveTextContent("SLO breach");
    expect(within(results).getByTestId("score")).toHaveTextContent("35 / 70");
    const rows = within(results).getAllByTestId("test-result");
    expect(rows).toHaveLength(2);
    expect(rows[1]).toHaveTextContent("Link goes viral");
    expect(within(rows[1]).getByTestId("bottleneck")).toHaveTextContent("SQL Database saturated from 22 s.");
    expect(within(results).getByTestId("ai-review")).toHaveTextContent("arrives in M4");
    await user.click(within(rows[1]).getByRole("button", { name: "Replay at failure" }));
    await waitFor(() => expect(fake().runs).toBe(1));
    const [, options] = vi.mocked(fake().api.startRun).mock.calls[0];
    expect(options.problem).toEqual({ problem_id: "url-shortener", test_id: "viral" });
    expect(options.startS).toBe(20);
    expect(screen.queryByTestId("results")).not.toBeInTheDocument();
    act(() => useRun.getState().stop());
  });

  it("a first Accepted submission says so and refreshes the brief", async () => {
    const { user } = renderApp("/problems/url-shortener");
    await screen.findByTestId("brief");
    await user.click(screen.getByRole("button", { name: "Submit" }));
    const results = await screen.findByTestId("results");
    expect(results).toHaveTextContent("Solved! The editorial is now in the Brief tab.");
    expect(within(results).getByTestId("score")).toHaveTextContent("95");
    await user.click(within(results).getAllByRole("button", { name: "Close" }).at(-1)!);
    expect((await screen.findAllByRole("heading", { name: "Editorial" })).length).toBeGreaterThan(0);
  });

  it("edits autosave the draft", async () => {
    renderApp("/problems/url-shortener");
    await screen.findByTestId("brief");
    act(() => useEditor.getState().rename("My shortener"));
    await waitFor(() => expect(fake().api.saveDraft).toHaveBeenCalled(), { timeout: 2000 });
    const [id, design] = vi.mocked(fake().api.saveDraft).mock.calls.at(-1)!;
    expect(id).toBe("url-shortener");
    expect(design.name).toBe("My shortener");
  });

  it("Run plays the problem's visible test", async () => {
    const { user } = renderApp("/problems/url-shortener");
    await screen.findByTestId("brief");
    await user.click(screen.getByRole("button", { name: "Run" }));
    await waitFor(() => expect(fake().runs).toBe(1));
    expect(vi.mocked(fake().api.startRun).mock.calls[0][1].problem).toEqual({ problem_id: "url-shortener", test_id: "normal" });
    act(() => useRun.getState().stop());
  });
});
