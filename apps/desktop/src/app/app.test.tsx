import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it, vi } from "vitest";
import { act, fireEvent, screen, waitFor, within } from "@testing-library/react";
import type { Design } from "@/bindings";
import { setFakeApi, useFakeApi } from "@/test/mockApi";
import { renderApp } from "@/test/renderApp";
import { unnamed } from "@/test/a11y";
import { useEditor } from "@/solve/store";
import { REGISTRY } from "@/test/fakeApi";
import { useRun } from "@/solve/run/runStore";
import { useTelemetry } from "@/solve/run/telemetry";

vi.mock("@/lib/api", async () => (await import("@/test/mockApi")).apiModule);
const fake = useFakeApi();

const design: Design = {
  schema_version: 1,
  id: "design-1",
  name: "URL Shortener",
  problem_id: null,
  nodes: [
    { id: "c1", kind: "client", flavour: "web", label: "", position: { x: 0, y: 0 }, props: {}, group_id: null },
    { id: "a1", kind: "app", flavour: "go", label: "API", position: { x: 300, y: 0 }, props: {}, group_id: null },
  ],
  edges: [{ id: "e1", from: "c1", to: "a1", protocol: "http" }],
  groups: [],
  notes: [],
  viewport: { x: 0, y: 0, zoom: 1 },
};

describe("pages", () => {
  it.each(["/", "/problems", "/guide", "/samples", "/progress", "/settings"])(
    "%s renders with named controls and a main landmark",
    async (path) => {
      renderApp(path);
      expect(await screen.findByRole("main")).toBeInTheDocument();
      expect(await screen.findByRole("heading", { level: 1 })).toBeInTheDocument();
      expect(unnamed(document.body)).toEqual([]);
    },
  );

  it("the Sandbox renders the library, the canvas and the inspector, with named controls", async () => {
    renderApp("/sandbox");
    expect(await screen.findByTestId("library")).toBeInTheDocument();
    expect(screen.getByTestId("canvas")).toBeInTheDocument();
    expect(screen.getByTestId("inspector")).toBeInTheDocument();
    expect(screen.getAllByTestId("library-row")).toHaveLength(42 + 3);
    expect(unnamed(document.body)).toEqual([]);
  });

  it("shows the DEV DATA badge in development builds only", async () => {
    renderApp("/");
    expect(await screen.findByTestId("dev-data-badge")).toHaveTextContent("DEV DATA");
  });

  it("later features are disabled with a milestone hint", async () => {
    renderApp("/sandbox");
    await screen.findByTestId("library");
    expect(screen.getByRole("button", { name: /^Submit/ })).toBeDisabled();
    expect(screen.getByLabelText("AI review: Arrives in M4")).toBeInTheDocument();
    // Run arrived in M1.
    expect(screen.getByRole("button", { name: "Run" })).toBeEnabled();
  });
});

describe("library", () => {
  it("search filters by kind and product names, accents ignored", async () => {
    const { user } = renderApp("/sandbox");
    const search = await screen.findByRole("searchbox", { name: "Search the library" });
    await user.type(search, "redis");
    const rows = screen.getAllByTestId("library-row").map((r) => r.getAttribute("data-kind"));
    expect(rows).toEqual(["ratelimit", "cache", "pubsub"]);
    await user.clear(search);
    await user.type(search, "cola");
    expect(screen.queryAllByTestId("library-row")).toHaveLength(0);
    expect(screen.getByText("No component matches.")).toBeInTheDocument();
  });

  it("categories collapse and show their counts", async () => {
    const { user } = renderApp("/sandbox");
    const header = await screen.findByRole("button", { name: "Clients & Edge · 10" });
    expect(header).toHaveAttribute("aria-expanded", "true");
    await user.click(header);
    expect(header).toHaveAttribute("aria-expanded", "false");
    expect(screen.getAllByTestId("library-row")).toHaveLength(42 + 3 - 10);
  });

  it("Enter on a row opens the flavour picker with the default preselected, then adds the card", async () => {
    const { user } = renderApp("/sandbox");
    const row = (await screen.findAllByTestId("library-row")).find((r) => r.dataset.kind === "cache")!;
    row.focus();
    await user.keyboard("{Enter}");
    const picker = await screen.findByTestId("floating-picker");
    expect(within(picker).getByRole("option", { selected: true })).toHaveTextContent("Redis");
    await user.keyboard("{ArrowDown}{Enter}");
    await waitFor(() => expect(useEditor.getState().doc.nodes).toHaveLength(1));
    expect(useEditor.getState().doc.nodes[0]).toMatchObject({ kind: "cache", flavour: "memcached" });
    // The new card is selected, so the inspector edits it.
    expect(await screen.findByTestId("inspector-node")).toBeInTheDocument();
  });

  it("Autoscaling from the library places a wrapper with its policy, not a card", async () => {
    const { user } = renderApp("/sandbox");
    const row = (await screen.findAllByTestId("library-row")).find((r) => r.dataset.kind === "autoscale")!;
    row.focus();
    await user.keyboard("{Enter}");
    await screen.findByTestId("floating-picker");
    await user.keyboard("{Enter}");
    await waitFor(() => expect(useEditor.getState().doc.groups).toHaveLength(1));
    const g = useEditor.getState().doc.groups[0];
    expect(g.kind).toBe("autoscale");
    // Sparse props (v3): the policy follows the Autoscaling defaults until edited.
    expect(g.scaling).toEqual({ flavour: "kubernetes_hpa", props: {} });
    expect(useEditor.getState().doc.nodes).toHaveLength(0);
    expect(await screen.findByText("Scaling policy")).toBeInTheDocument();
  });

  it("the inspector shows the flavour's defaults and resets a prop the user set", async () => {
    const { user } = renderApp("/sandbox");
    const id = useEditor.getState().addNode(REGISTRY, "app", "go", { x: 0, y: 0 });
    useEditor.getState().setSelection([id]);
    const panel = await screen.findByTestId("inspector-node");
    // Go's service time (5 ms), not the kind's 10, though the card stores nothing.
    expect(within(panel).getByLabelText(/Service time/)).toHaveValue(5);
    expect(within(panel).queryByTestId("prop-reset")).toBeNull();
    const instances = within(panel).getByLabelText(/Instances/);
    await user.clear(instances);
    await user.type(instances, "7{Enter}");
    expect(useEditor.getState().doc.nodes[0].props).toEqual({ instances: 7 });
    await user.click(within(panel).getByTestId("prop-reset"));
    expect(useEditor.getState().doc.nodes[0].props).toEqual({});
    expect(within(panel).getByLabelText(/Instances/)).toHaveValue(2);
  });

  it("the info button opens the quick look", async () => {
    const { user } = renderApp("/sandbox");
    await user.click(await screen.findByRole("button", { name: "About Queue" }));
    const look = await screen.findByTestId("quick-look");
    expect(within(look).getByText("RabbitMQ")).toBeInTheDocument();
    // No Guide entry for this kind in the fake backend: the quick look still links to it.
    expect(await within(look).findByRole("link", { name: "Open in Guide" })).toBeInTheDocument();
  });
});

describe("inspector", () => {
  it("edits a card's props with the registry's ranges and refuses values outside them", async () => {
    act(() => useEditor.getState().load(design, null));
    act(() => useEditor.getState().setSelection(["a1"]));
    const { user } = renderApp("/sandbox");
    const instances = await screen.findByLabelText("Instances");
    expect(instances).toHaveAttribute("min", "1");
    expect(instances).toHaveAttribute("max", "64");
    await user.clear(instances);
    await user.type(instances, "2.5{Enter}");
    expect(await screen.findByRole("alert")).toHaveTextContent("Enter a whole number from 1 to 64.");
    expect(useEditor.getState().doc.nodes[1].props.instances).toBeUndefined();
    await user.clear(instances);
    await user.type(instances, "6{Enter}");
    expect(useEditor.getState().doc.nodes[1].props.instances).toBe(6);
    const service = screen.getByLabelText("Service time (ms)");
    await user.clear(service);
    await user.type(service, "4.5{Enter}");
    expect(useEditor.getState().doc.nodes[1].props.service_ms).toBe(4.5);
  });

  it("tags calibrated defaults and leaves design choices untagged", async () => {
    act(() => useEditor.getState().load(design, null));
    act(() => useEditor.getState().setSelection(["a1"]));
    renderApp("/sandbox");
    await screen.findByLabelText("Instances");
    // An app server: concurrency and service time are calibrated, instances is a choice.
    const tags = screen.getAllByTestId("calibration-tag");
    expect(tags).toHaveLength(2);
    expect(tags[0]).toHaveTextContent("ballpark");
    expect(tags[0]).toHaveAttribute("title", expect.stringMatching(/source is still being confirmed/));
    expect(screen.getByText(/Tagged defaults are calibrated figures/)).toBeInTheDocument();
    expect(screen.queryByText(/placeholders until the simulator/)).not.toBeInTheDocument();
  });
});

describe("files", () => {
  it("opening a recent design shows it in the Sandbox", async () => {
    setFakeApi({
      files: { "C:/designs/short.kbarch": design },
      recent: [{ path: "C:/designs/short.kbarch", name: "URL Shortener", opened_at: 1, exists: true }],
    });
    const { user, history } = renderApp("/");
    await user.click(await screen.findByRole("button", { name: "Open URL Shortener" }));
    await waitFor(() => expect(history.location.pathname).toBe("/sandbox"));
    expect(useEditor.getState().doc.nodes).toHaveLength(2);
    expect(useEditor.getState().path).toBe("C:/designs/short.kbarch");
    expect(await screen.findByTestId("design-name")).toHaveTextContent("URL Shortener");
  });

  it("Save asks for a path the first time, then the title shows the file without the dirty marker", async () => {
    setFakeApi({ savePick: "C:/designs/new" });
    act(() => useEditor.getState().newDesign("New one"));
    const { user } = renderApp("/sandbox");
    await screen.findByTestId("library");
    act(() => void useEditor.getState().addNote({ x: 0, y: 0 }));
    await waitFor(() => expect(fake().title).toBe("New one • — KataBox Arch"));
    expect(fake().dirty).toBe(true);
    await user.click(screen.getByRole("button", { name: "Save" }));
    await waitFor(() => expect(fake().title).toBe("new.kbarch — KataBox Arch"));
    expect(fake().dirty).toBe(false);
    expect(fake().files.get("C:/designs/new.kbarch")?.notes).toHaveLength(1);
  });

  it("closing with unsaved changes asks first; Don't save closes", async () => {
    act(() => useEditor.getState().newDesign("Draft"));
    const { user } = renderApp("/");
    await screen.findByRole("main");
    act(() => void useEditor.getState().addNote({ x: 0, y: 0 }));
    await waitFor(() => expect(fake().api.onCloseRequested).toHaveBeenCalled());
    await act(async () => {
      fake().requestClose();
    });
    const dialog = await screen.findByTestId("unsaved-dialog");
    expect(fake().api.confirmClose).not.toHaveBeenCalled();
    await user.click(within(dialog).getByRole("button", { name: "Don't save" }));
    await waitFor(() => expect(fake().api.confirmClose).toHaveBeenCalled());
  });

  it("New with unsaved changes can be cancelled", async () => {
    act(() => useEditor.getState().load(design, null));
    act(() => void useEditor.getState().addNote({ x: 0, y: 0 }));
    const { user } = renderApp("/sandbox");
    await user.click(await screen.findByRole("button", { name: "New design" }));
    const dialog = await screen.findByTestId("unsaved-dialog");
    await user.click(within(dialog).getByRole("button", { name: "Cancel" }));
    expect(useEditor.getState().doc.id).toBe("design-1");
  });
});

describe("command palette", () => {
  it("Ctrl+K opens it; a theme command applies", async () => {
    const { user } = renderApp("/");
    await screen.findByRole("main");
    fireEvent.keyDown(window, { key: "k", ctrlKey: true });
    const input = await screen.findByPlaceholderText("Type a command, a page or a component…");
    await user.type(input, "theme dark");
    await user.keyboard("{Enter}");
    await waitFor(() => expect(document.documentElement).toHaveClass("dark"));
    expect(fake().api.setSetting).toHaveBeenCalledWith("theme", "dark");
  });

  it("adds a component to the canvas from anywhere", async () => {
    act(() => useEditor.getState().newDesign("Empty"));
    const { user, history } = renderApp("/");
    await screen.findByRole("main");
    fireEvent.keyDown(window, { key: "k", ctrlKey: true });
    await user.type(await screen.findByRole("combobox"), "add load balancer");
    await user.keyboard("{Enter}");
    await waitFor(() => expect(history.location.pathname).toBe("/sandbox"));
    await waitFor(() => expect(useEditor.getState().doc.nodes.map((n) => n.kind)).toEqual(["lb"]));
  });
});

describe("settings", () => {
  it("switches language and contrast, saving both", async () => {
    const { user } = renderApp("/settings");
    await user.click(await screen.findByRole("radio", { name: "Español" }));
    expect(await screen.findByRole("heading", { level: 1, name: "Ajustes" })).toBeInTheDocument();
    await user.click(screen.getByRole("radio", { name: "Alto" }));
    expect(document.documentElement).toHaveClass("hc");
    expect(fake().api.setSetting).toHaveBeenCalledWith("ui_lang", "es");
    expect(fake().api.setSetting).toHaveBeenCalledWith("ui.contrast", "high");
  });
});

describe("live runs", () => {
  // A frame Rust encoded (see solve/run/frames.test.ts).
  const frame = () => {
    const b = readFileSync(resolve(__dirname, "../test/fixtures/frame.kaf"));
    return b.buffer.slice(b.byteOffset, b.byteOffset + b.byteLength);
  };

  it("Run starts the design, the Metrics tab shows the live tiles, Pause pauses", async () => {
    act(() => useEditor.getState().load(design, null));
    const { user } = renderApp("/sandbox");
    await user.click(await screen.findByRole("button", { name: "Run" }));
    await waitFor(() => expect(fake().api.startRun).toHaveBeenCalledTimes(1));
    const [sent, options] = vi.mocked(fake().api.startRun).mock.calls[0];
    expect(sent.nodes.map((n) => n.id)).toEqual(["c1", "a1"]);
    expect(sent.schema_version).toBe(4);
    expect(options).toEqual({ preset: "normal", problem: null, speed: 1, chaos: [], startS: 0 });
    expect(screen.getByRole("tab", { name: "Metrics" })).toHaveAttribute("aria-selected", "true");
    expect(screen.getByText("Run the design to see live metrics.")).toBeInTheDocument();
    act(() => fake().pushFrame(frame()));
    expect(await screen.findByTestId("metric-rps")).toHaveTextContent(/req\/s/);
    expect(screen.getByTestId("metric-p95")).toHaveTextContent(/ms$/);
    expect(screen.getByTestId("run-clock")).toHaveTextContent("3.0 s of 60 s");
    await user.click(screen.getByRole("button", { name: "Pause" }));
    expect(fake().runControls).toEqual(["pause"]);
    expect(screen.getByRole("button", { name: "Resume" })).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Stop" }));
    expect(fake().runControls).toEqual(["pause", "stop"]);
    expect(useRun.getState().status).toBe("idle");
  });

  it("editing what the simulator reads restarts the run; moving a card doesn't", async () => {
    act(() => useEditor.getState().load(design, null));
    const { user } = renderApp("/sandbox");
    await user.click(await screen.findByRole("button", { name: "Run" }));
    await waitFor(() => expect(fake().runs).toBe(1));
    act(() => fake().pushFrame(frame()));
    act(() => useEditor.getState().moveItems([{ id: "a1", position: { x: 400, y: 40 } }]));
    act(() => useEditor.getState().setProp("a1", "instances", 4));
    // The old metrics stay on screen while the edited design restarts.
    await waitFor(() => expect(fake().runs).toBe(2), { timeout: 2000 });
    expect(screen.getByTestId("metric-rps")).toBeInTheDocument();
    const restarted = vi.mocked(fake().api.startRun).mock.calls[1][0];
    expect(restarted.nodes[1].props.instances).toBe(4);
    act(() => useRun.getState().stop());
  });

  it("cards show their live load; with reduced motion, connections show their throughput instead of packets", async () => {
    // The app's motion setting (its local mirror is read at start-up).
    localStorage.setItem("katabox-arch.ui.motion", "reduced");
    try {
      act(() => useEditor.getState().load(design, null));
      const { user } = renderApp("/sandbox");
      await user.click(await screen.findByRole("button", { name: "Run" }));
      await waitFor(() => expect(fake().runs).toBe(1));
      expect(screen.queryByTestId("packet-layer")).not.toBeInTheDocument();
      act(() => fake().pushFrame(frame()));
      // The app (frame node 1 → card a1) is busy; health refreshes within 100 ms.
      const stats = await screen.findAllByTestId("card-stat");
      // The full stat is the tooltip and the screen-reader text; the card shows a short form (F5).
      expect(stats.map((s) => s.getAttribute("title"))).toEqual([expect.stringMatching(/^0% busy/), expect.stringMatching(/^\d+% busy/)]);
      expect(stats.map((s) => s.querySelector("[aria-hidden]")?.textContent)).toEqual(["0%", expect.stringMatching(/^\d+%($| · q\d+$)/)]);
      // React Flow draws no edges in jsdom: the chip's number comes from here (checked in the real app).
      expect(useTelemetry.getState().edgeRates.e1).toBeGreaterThan(0);
      act(() => useRun.getState().stop());
      await waitFor(() => expect(screen.queryByTestId("card-stat")).not.toBeInTheDocument());
    } finally {
      localStorage.removeItem("katabox-arch.ui.motion");
      document.documentElement.classList.remove("reduce-motion");
    }
  });

  it("the packet layer is on the canvas when motion is allowed", async () => {
    act(() => useEditor.getState().load(design, null));
    renderApp("/sandbox");
    expect(await screen.findByTestId("packet-layer")).toBeInTheDocument();
  });

  it("Inject kills the selected card at the current moment and the run continues from there", async () => {
    act(() => useEditor.getState().load(design, null));
    const { user } = renderApp("/sandbox");
    await user.click(await screen.findByRole("button", { name: "Run" }));
    await waitFor(() => expect(fake().runs).toBe(1));
    act(() => fake().pushFrame(frame()));
    act(() => useEditor.getState().setSelection(["a1"]));
    await user.click(screen.getByRole("button", { name: "Inject" }));
    await user.click(await screen.findByRole("menuitem", { name: "Kill it" }));
    await waitFor(() => expect(fake().runs).toBe(2));
    const { chaos, startS } = vi.mocked(fake().api.startRun).mock.calls[1][1];
    expect(chaos).toEqual([{ kill: { id: "a1", at: 2.999 } }]);
    expect(startS).toBe(2.999);
    // The timeline marks it.
    expect(await screen.findByTestId("chaos-marker")).toHaveAttribute("aria-label", "A card dies at 3.0 s");
    act(() => useRun.getState().stop());
  });

  it("the scrubber seeks when released, and the run pauses there", async () => {
    act(() => useEditor.getState().load(design, null));
    const { user } = renderApp("/sandbox");
    await user.click(await screen.findByRole("button", { name: "Run" }));
    await waitFor(() => expect(fake().runs).toBe(1));
    act(() => fake().pushFrame(frame()));
    const scrub = screen.getByRole("slider", { name: "Simulated time" });
    fireEvent.change(scrub, { target: { value: "42" } });
    expect(screen.getByTestId("run-clock")).toHaveTextContent("42.0 s of 60 s");
    fireEvent.pointerUp(scrub);
    expect(fake().runControls).toEqual([{ seek: 42 }]);
    expect(useRun.getState().status).toBe("paused");
    act(() => useRun.getState().stop());
  });

  it("Find breaking point reports the limit and the first card to give way, and goes stale after an edit", async () => {
    act(() => useEditor.getState().load(design, null));
    const { user } = renderApp("/sandbox");
    await user.click(await screen.findByRole("tab", { name: "Metrics" }));
    await user.click(screen.getByRole("button", { name: "Find breaking point" }));
    const result = await screen.findByTestId("breaking-result");
    expect(result).toHaveTextContent("Holds about 31,250 req/s (31.3× the clients' traffic).");
    expect(result).toHaveTextContent("API gives way first: every server busy within 5 s of the failing run.");
    expect(result).toHaveTextContent("At that load, 1.2% of reads are stale.");
    act(() => useEditor.getState().setProp("a1", "instances", 6));
    expect(await screen.findByText("The design has changed since; find it again.")).toBeInTheDocument();
  });

  it("a design with no path reports why instead of running", async () => {
    setFakeApi({ runError: "no_path" });
    act(() => useEditor.getState().load(design, null));
    const { user } = renderApp("/sandbox");
    await user.click(await screen.findByRole("button", { name: "Run" }));
    expect(await screen.findByText(/No request can get through this design/)).toBeInTheDocument();
    expect(useRun.getState().status).toBe("idle");
  });
});
