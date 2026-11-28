import { describe, expect, it, vi } from "vitest";
import type { Design } from "@/bindings";
import { act, screen, waitFor, within } from "@testing-library/react";
import { setFakeApi, useFakeApi } from "@/test/mockApi";
import { renderApp } from "@/test/renderApp";
import { unnamed } from "@/test/a11y";
import { useEditor } from "@/solve/store";
import { isActive, useRun } from "@/solve/run/runStore";

vi.mock("@/lib/api", async () => (await import("@/test/mockApi")).apiModule);

const FAKE_DESIGN: Design = {
  schema_version: 2,
  id: "d1",
  name: "D",
  problem_id: null,
  nodes: [{ id: "a1", kind: "app", flavour: "go", label: "", position: { x: 0, y: 0 }, props: {}, group_id: null }],
  edges: [],
  groups: [],
  notes: [],
  viewport: { x: 0, y: 0, zoom: 1 },
};
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

  it("heads the Core list with its sets (H6)", async () => {
    renderApp("/problems");
    await screen.findByTestId("problem-row");
    const headings = screen.getAllByTestId("set-heading");
    expect(headings[0]).toHaveTextContent("Set 1");
    expect(within(headings[0]!).getByRole("heading", { level: 2 })).toBeInTheDocument();
  });

  it("shows one row of topic tags until asked for more (H6)", async () => {
    const list = await fake().api.listProblems();
    const tags = Array.from({ length: 15 }, (_, i) => `t${String(i).padStart(2, "0")}`);
    vi.mocked(fake().api.listProblems).mockResolvedValue(list.map((p, i) => (i === 0 ? { ...p, tags } : p)));
    const { user } = renderApp("/problems");
    const cloud = await screen.findByTestId("tag-cloud");
    await waitFor(() => expect(within(cloud).getByRole("button", { name: "#t00" })).toBeInTheDocument());
    expect(within(cloud).queryByRole("button", { name: "#t14" })).not.toBeInTheDocument();
    const more = within(cloud).getByTestId("more-tags");
    expect(more).toHaveAttribute("aria-expanded", "false");
    await user.click(more);
    expect(within(cloud).getByRole("button", { name: "#t14" })).toBeInTheDocument();
    expect(more).toHaveTextContent("Fewer tags");
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
    // A problem with a p99 limit shows it with the targets.
    expect(brief).toHaveTextContent("p99 latency≤ 120 ms");
    const tests = within(brief).getByTestId("tests");
    expect(tests).toHaveTextContent("Link goes viral");
    expect(tests).toHaveTextContent("hidden");
    // A visible test with its own availability limit says so (M7b.4).
    expect(tests).toHaveTextContent("60 s · availability ≥ 99%");
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
    // A p99 cell only where the test has a p99 limit.
    expect(within(rows[0]).queryByTestId("p99")).toBeNull();
    expect(within(rows[1]).getByTestId("p99")).toHaveTextContent("900 ms / 120 ms");
    // Each test shows its availability limit, and lost messages against theirs (M7b.4).
    expect(within(rows[1]).getByTestId("availability")).toHaveTextContent("96.6% / ≥ 99.9%");
    expect(within(rows[0]).queryByTestId("lost")).toBeNull();
    expect(within(rows[1]).getByTestId("lost")).toHaveTextContent("12 messages lost (limit 0)");
    // The cost says which test's load and which prices it uses.
    expect(within(results).getByTestId("cost-note")).toHaveTextContent("Monthly cost at the load of “Normal day”, at list prices");
    // Retried attempts are their own number, not errors (review F3).
    expect(within(rows[0]).queryByTestId("retried")).toBeNull();
    expect(within(rows[1]).getByTestId("retried")).toHaveTextContent("1,234 attempts failed and were retried: not errors");
    expect(within(rows[1]).getByTestId("bottleneck")).toHaveTextContent("SQL Database saturated from 22 s.");
    expect(within(results).getByTestId("ai-review")).toHaveTextContent("arrives in M4");
    await user.click(within(rows[1]).getByRole("button", { name: "Replay at failure" }));
    await waitFor(() => expect(fake().runs).toBe(1));
    const [, options] = vi.mocked(fake().api.startRun).mock.calls[0];
    expect(options.problem).toEqual({ problem_id: "url-shortener", test_id: "viral" });
    expect(options.startS).toBe(20);
    expect(screen.queryByTestId("results")).not.toBeInTheDocument();
    // The run keeps going once the dialog has closed and the card is selected, and the
    // toolbar names the hidden test it replays.
    await waitFor(() => expect(useEditor.getState().selection).toEqual(["db"]));
    await act(() => new Promise((r) => setTimeout(r, 50)));
    expect(isActive(useRun.getState().status)).toBe(true);
    expect(screen.getByRole("button", { name: "Stop" })).toBeInTheDocument();
    act(() => useRun.getState().stop());
    expect(screen.getByRole("combobox", { name: "Test to run" })).toHaveTextContent("Link goes viral");
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

  it("opening a design made for a problem opens it on that problem's page", async () => {
    const mine = { ...structuredClone(FAKE_DESIGN), problem_id: "url-shortener", name: "My shortener" };
    setFakeApi({ files: { "C:/designs/mine.kbarch": mine }, openPick: "C:/designs/mine.kbarch" });
    const { user, history } = renderApp("/");
    await user.click(await screen.findByRole("button", { name: /Open/ }));
    await waitFor(() => expect(history.location.pathname).toBe("/problems/url-shortener"));
    expect(await screen.findByTestId("brief")).toBeInTheDocument();
    expect(useEditor.getState().doc.name).toBe("My shortener");
    expect(useEditor.getState().doc.nodes).toHaveLength(1);
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

  it("a failed submission offers the next hint in the results", async () => {
    setFakeApi({ verdict: "slo_breach" });
    const { user } = renderApp("/problems/url-shortener");
    await screen.findByTestId("brief");
    await user.click(screen.getByRole("button", { name: "Submit" }));
    const hint = await within(await screen.findByTestId("results")).findByTestId("results-hint");
    await user.click(within(hint).getByRole("button", { name: "Show hint 1" }));
    expect(await within(hint).findByText("Look at the reads.")).toBeInTheDocument();
    expect(within(hint).getByRole("button", { name: "Show hint 2" })).toBeInTheDocument();
    expect(within(screen.getByTestId("results")).queryByRole("button", { name: "Compare with reference" })).toBeNull();
  });

  it("once solved, Compare shows both designs side by side or overlaid, and the chart sums up the points", async () => {
    const { user } = renderApp("/problems/url-shortener");
    await screen.findByTestId("brief");
    await user.click(screen.getByRole("button", { name: "Submit" }));
    const results = await screen.findByTestId("results");
    expect(within(results).queryByTestId("results-hint")).toBeNull();
    await user.click(await within(results).findByRole("button", { name: "Compare with reference" }));
    const compare = await screen.findByTestId("compare");
    expect(within(compare).getByRole("heading", { name: "Your design and the reference" })).toBeInTheDocument();
    const mine = within(compare).getByTestId("compare-mine");
    const reference = within(compare).getByTestId("compare-reference");
    expect(within(reference).getAllByTestId("component-card")).toHaveLength(2);
    expect(reference).toHaveTextContent("Redis");
    expect(within(mine).queryAllByTestId("component-card")).toHaveLength(useEditor.getState().doc.nodes.length);
    await user.click(within(compare).getByRole("radio", { name: "Overlay" }));
    expect(within(compare).getByTestId("compare-ghost")).toBeInTheDocument();
    expect(within(compare).getByText("Reference: faint")).toBeInTheDocument();
    expect(unnamed(compare)).toEqual([]);
    await user.keyboard("{Escape}");
    await waitFor(() => expect(screen.queryByTestId("compare")).not.toBeInTheDocument());
    await user.keyboard("{Escape}");
    // The Brief's solved section: Compare again, and where the accepted designs sit.
    const brief = await screen.findByTestId("brief");
    expect(await within(brief).findByTestId("chart-best")).toHaveTextContent("Your cheapest accepted design: $1,850 a month at 31.2 ms p95 (1 accepted).");
    expect(within(brief).getByTestId("chart-reference")).toHaveTextContent("The reference: $1,600 a month at 24.5 ms p95.");
    expect(within(brief).getByRole("button", { name: "Compare with reference" })).toBeInTheDocument();
  });
});

describe("pinned model limits (H2)", () => {
  it("shows a hosted model's account limits read-only when the problem sets them", async () => {
    const base = await fake().api.problemBrief("url-shortener");
    const draft: Design = {
      ...FAKE_DESIGN,
      problem_id: "url-shortener",
      nodes: [
        { id: "m1", kind: "infer", flavour: "claude_api", label: "", position: { x: 0, y: 0 }, props: { rpm: 99999 }, group_id: null },
        { id: "m2", kind: "infer", flavour: "openai_api", label: "", position: { x: 300, y: 0 }, props: {}, group_id: null },
      ],
    };
    vi.mocked(fake().api.problemBrief).mockResolvedValue({ ...base, draft, model_limits: [{ flavour: "claude_api", rpm: 4000, tpm: 400000 }] });
    const { user } = renderApp("/problems/url-shortener");
    await screen.findByTestId("brief");
    await waitFor(() => expect(useEditor.getState().doc.nodes).toHaveLength(2));
    act(() => useEditor.getState().setSelection(["m1"]));
    await user.click(screen.getByRole("tab", { name: "Inspect" }));
    const pinned = await screen.findAllByTestId("prop-pinned");
    expect(pinned.map((p) => (p as HTMLInputElement).value)).toEqual(["4000", "400000"]);
    expect(pinned[0]).toHaveAttribute("readonly");
    expect(pinned[0]).toBeDisabled();
    expect(screen.getAllByText("set by the problem")).toHaveLength(2);
    // F5: a hosted model has no GPU instances or batch slots to set.
    const inspector = screen.getByTestId("inspector-node");
    expect(within(inspector).queryByText("Instances")).not.toBeInTheDocument();
    expect(within(inspector).queryByText("Batch slots per GPU")).not.toBeInTheDocument();
    // Another provider's card keeps its own, editable limits.
    act(() => useEditor.getState().setSelection(["m2"]));
    await waitFor(() => expect(screen.queryByTestId("prop-pinned")).not.toBeInTheDocument());
  });
});

describe("the run's test picker (F5)", () => {
  it("shows and runs the first visible test when none is chosen", async () => {
    renderApp("/problems/url-shortener");
    await screen.findByTestId("brief");
    act(() => useRun.getState().setProblemTest(null));
    const picker = await screen.findByRole("combobox", { name: "Test to run" });
    const brief = await fake().api.problemBrief("url-shortener");
    const first = brief.tests.find((x) => !x.hidden) ?? brief.tests[0];
    expect(picker).toHaveTextContent(first.name);
  });
});

describe("opening a draft (H6)", () => {
  it("asks the canvas to fit the draft in view", async () => {
    const base = await fake().api.problemBrief("url-shortener");
    const draft: Design = { ...FAKE_DESIGN, problem_id: "url-shortener" };
    vi.mocked(fake().api.problemBrief).mockResolvedValue({ ...base, draft });
    act(() => useEditor.getState().newDesign("Other"));
    const before = useEditor.getState().fitRequest;
    renderApp("/problems/url-shortener");
    await screen.findByTestId("brief");
    await waitFor(() => expect(useEditor.getState().fitRequest).toBe(before + 1));
  });
});
