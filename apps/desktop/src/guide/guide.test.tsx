import { describe, expect, it, vi } from "vitest";
import { act, screen, waitFor, within } from "@testing-library/react";
import { useFakeApi } from "@/test/mockApi";
import { renderApp } from "@/test/renderApp";
import { unnamed } from "@/test/a11y";
import { useEditor } from "@/solve/store";

vi.mock("@/lib/api", async () => (await import("@/test/mockApi")).apiModule);

const fake = useFakeApi();

describe("the Component Guide", () => {
  it("the index groups entries, searches, and leads to an entry", async () => {
    const { user, history } = renderApp("/guide");
    expect(await screen.findByRole("heading", { name: "Component Guide" })).toBeInTheDocument();
    expect(await screen.findByTestId("guide-progress")).toHaveTextContent("0 of 2 read");
    const start = screen.getByTestId("start-here");
    expect(within(start).getByRole("link", { name: /Cache/ })).toBeInTheDocument();
    expect(within(start).getByRole("link", { name: /Load Balancer/ })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Clients & Edge" })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Data" })).toBeInTheDocument();
    expect(screen.getAllByTestId("guide-tile")).toHaveLength(2);
    await user.type(screen.getByRole("searchbox", { name: "Search the Guide" }), "memory");
    expect(screen.getAllByTestId("guide-tile")).toHaveLength(1);
    expect(unnamed(document.body)).toEqual([]);
    await user.click(screen.getByTestId("guide-tile"));
    await waitFor(() => expect(history.location.pathname).toBe("/guide/cache"));
  });

  it("an entry shows the registry's numbers per product, pitfalls and practice, and can be marked read", async () => {
    const { user, history } = renderApp("/guide/cache");
    const entry = await screen.findByTestId("guide-entry");
    expect(within(entry).getByRole("heading", { level: 1, name: "Cache" })).toBeInTheDocument();
    expect(entry).toHaveTextContent("A fast, in-memory copy of the data you read most.");
    expect(within(entry).getByText("hot").tagName).toBe("STRONG");
    // In the simulator: one column per flavour, values from the registry fixture.
    const sim = within(entry).getByTestId("sim-table");
    const rows = within(sim).getAllByRole("row");
    expect(rows[0]).toHaveTextContent("Redis");
    expect(rows[0]).toHaveTextContent("Memcached");
    expect(rows[1]).toHaveTextContent("Memory");
    expect(rows[1]).toHaveTextContent("2 GB");
    expect(rows[2]).toHaveTextContent("Parallel work per node");
    expect(within(rows[2]).getAllByRole("cell").map((c) => c.textContent)).toEqual(["1", "4", "Parallel work per node."]);
    expect(rows[3]).toHaveTextContent("$18 per GB a month");
    expect(within(entry).getByTestId("flavours-compared")).toHaveTextContent("Multi-threaded");
    // Seen in (the pitfall) and Practice it both link to the problem.
    await waitFor(() => expect(within(entry).getAllByRole("link", { name: /^URL Shortener/ })).toHaveLength(2));
    expect(within(entry).getByTestId("interview-tip")).toHaveTextContent("Say what you cache and for how long.");
    expect(entry).toHaveTextContent("https://redis.io/docs/latest/");
    expect(unnamed(entry)).toEqual([]);

    await user.click(within(entry).getByRole("button", { name: "Mark as read" }));
    expect(await within(entry).findByRole("button", { name: "Read" })).toHaveAttribute("aria-pressed", "true");
    expect(fake().api.setGuideRead).toHaveBeenCalledWith("cache", true);
    await user.click(within(entry).getByRole("link", { name: "App Server" }));
    await waitFor(() => expect(history.location.pathname).toBe("/guide/app"));
    act(() => void history.push("/guide"));
    expect(await screen.findByTestId("guide-progress")).toHaveTextContent("1 of 2 read");
  });

  it("an entry with a concept card plays it inline", async () => {
    const { user } = renderApp("/guide/cache");
    const card = await screen.findByTestId("concept-card");
    await user.click(within(card).getByRole("button", { name: "Play the concept card (1 step)" }));
    const player = await screen.findByTestId("concept-player");
    expect(within(player).getByTestId("step-counter")).toHaveTextContent("step 1 of 1");
    await waitFor(() => expect(fake().runs).toBe(1));
    await user.click(within(player).getByRole("button", { name: "Close" }));
    expect(screen.queryByTestId("concept-player")).toBeNull();
    expect(screen.getByTestId("concept-card")).toBeInTheDocument();
  });

  it("an entry without a concept card shows none", async () => {
    renderApp("/guide/lb");
    await screen.findByTestId("guide-entry");
    expect(screen.queryByTestId("concept-card")).toBeNull();
  });

  it("an unknown kind says the Guide is unavailable", async () => {
    renderApp("/guide/dns");
    expect(await screen.findByText("The Guide is unavailable")).toBeInTheDocument();
  });
});

describe("the Guide from the editor", () => {
  it("the library's quick look shows the entry in short and opens it", async () => {
    const { user, history } = renderApp("/sandbox");
    await user.click(await screen.findByRole("button", { name: "About Cache" }));
    const look = await screen.findByTestId("quick-look-entry");
    expect(look).toHaveTextContent("A fast, in-memory copy of the data you read most.");
    expect(look).toHaveTextContent("Reads far outnumber writes");
    expect(look).toHaveTextContent("Watch for: A popular key expires");
    await user.click(within(look).getByRole("link", { name: "Open in Guide" }));
    await waitFor(() => expect(history.location.pathname).toBe("/guide/cache"));
  });

  it("the Inspector's Learn more opens the entry at In the simulator", async () => {
    const { user, history } = renderApp("/sandbox");
    await screen.findByTestId("canvas");
    act(() => {
      const s = useEditor.getState();
      s.load(
        {
          schema_version: 2,
          id: "d",
          name: "D",
          problem_id: null,
          nodes: [{ id: "c1", kind: "cache", flavour: "redis", label: "", position: { x: 0, y: 0 }, props: {}, group_id: null }],
          edges: [],
          groups: [],
          notes: [],
          viewport: { x: 0, y: 0, zoom: 1 },
        },
        null,
      );
      useEditor.getState().setSelection(["c1"]);
    });
    await user.click(await screen.findByRole("link", { name: "Learn more" }));
    await waitFor(() => expect(history.location.pathname).toBe("/guide/cache"));
    expect(history.location.search).toContain("section=sim");
    expect(await screen.findByTestId("sim-table")).toBeInTheDocument();
  });

  it("Ctrl+K finds Guide entries", async () => {
    const { user, history } = renderApp("/");
    await screen.findByTestId("home-samples");
    await user.keyboard("{Control>}k{/Control}");
    await user.type(await screen.findByRole("combobox"), "guide cache");
    await user.click(await screen.findByRole("option", { name: "Guide: Cache" }));
    await waitFor(() => expect(history.location.pathname).toBe("/guide/cache"));
  });
});
