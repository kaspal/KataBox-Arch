// Accessibility pass (M7.1), after KataBox's: every page has the landmarks and the skip
// link, and nothing interactive is left without a name a screen reader can read (an
// axe-like check without a new dependency). Tablists are named, and a selected tab
// controls a panel labelled by it.
import { describe, expect, it, vi } from "vitest";
import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { renderApp } from "@/test/renderApp";
import { setFakeApi, useFakeApi } from "@/test/mockApi";
import { unnamed } from "@/test/a11y";
import { act } from "@testing-library/react";
import type { Design } from "@/bindings";
import { useEditor } from "@/solve/store";
import { useRun } from "@/solve/run/runStore";

vi.mock("@/lib/api", async () => (await import("@/test/mockApi")).apiModule);

const PAGES = [
  "/",
  "/problems",
  "/problems/url-shortener",
  "/sandbox",
  "/guide",
  "/guide/cache",
  "/decide/sql-or-nosql",
  "/glossary",
  "/numbers",
  "/review",
  "/paths",
  "/flaws",
  "/estimate",
  "/patterns",
  "/patterns/cache-aside",
  "/samples",
  "/progress",
  "/settings",
];

describe("accessibility pass", () => {
  useFakeApi();

  it.each(PAGES)("%s: landmarks, the skip link, and every control named", async (path) => {
    setFakeApi({ due: ["guide:cache:1"] });
    renderApp(path);
    await waitFor(() => expect(document.querySelector("main#main")).not.toBeNull());
    // Let the page's data arrive before scanning.
    await waitFor(() => expect(document.querySelector("main#main")!.textContent!.trim().length).toBeGreaterThan(20));
    await new Promise((r) => setTimeout(r, 80));
    expect(document.querySelectorAll("main").length).toBe(1);
    expect(document.querySelector("nav[aria-label]")).not.toBeNull();
    expect(screen.getByTestId("skip-link")).toHaveAttribute("href", "#main");
    expect(unnamed(document.body), `unnamed controls on ${path}`).toEqual([]);
    for (const list of document.querySelectorAll('[role="tablist"]')) {
      if (list.closest("[hidden], [aria-hidden='true']")) continue;
      const named = list.getAttribute("aria-label") || list.getAttribute("aria-labelledby") || list.closest("nav[aria-label]");
      expect(named, `unnamed tablist on ${path}`).toBeTruthy();
    }
    for (const tab of document.querySelectorAll('main [role="tab"][aria-selected="true"]')) {
      const panel = document.getElementById(tab.getAttribute("aria-controls") ?? "");
      expect(panel?.getAttribute("role"), `tab panel for ${tab.textContent} on ${path}`).toBe("tabpanel");
      expect(panel?.getAttribute("aria-labelledby")).toBe(tab.id);
    }
  });

  it("the side panel's tabs follow the ARIA tabs pattern", async () => {
    setFakeApi({});
    renderApp("/sandbox");
    const inspect = await screen.findByRole("tab", { name: "Inspect" });
    expect(screen.getByRole("tablist", { name: "Side panel" })).toBeInTheDocument();
    expect(inspect).toHaveAttribute("tabindex", "0");
    const user = userEvent.setup();
    inspect.focus();
    await user.keyboard("{ArrowRight}");
    const metrics = screen.getByRole("tab", { name: "Metrics" });
    expect(metrics).toHaveAttribute("aria-selected", "true");
    expect(metrics).toHaveFocus();
    expect(inspect).toHaveAttribute("tabindex", "-1");
    await user.keyboard("{Home}");
    expect(inspect).toHaveFocus();
  });

  it("cards have full names and run changes are announced", async () => {
    const design: Design = {
      schema_version: 3,
      id: "d",
      name: "D",
      problem_id: null,
      nodes: [
        { id: "c1", kind: "client", flavour: "web", label: "Users", position: { x: 0, y: 0 }, props: {}, group_id: null },
        { id: "a1", kind: "app", flavour: "go", label: "API", position: { x: 0, y: 200 }, props: { instances: 3 }, group_id: null },
      ],
      edges: [{ id: "e1", from: "c1", to: "a1", protocol: "http" }],
      groups: [],
      notes: [],
      viewport: { x: 0, y: 0, zoom: 1 },
    };
    setFakeApi({});
    act(() => useEditor.getState().load(design, null));
    const { user } = renderApp("/sandbox");
    expect(await screen.findByLabelText("API, Go App Server, 3 instances")).toBeInTheDocument();
    expect(screen.getByTestId("run-announcer")).toHaveTextContent("");
    await user.click(screen.getByRole("button", { name: "Run" }));
    await waitFor(() => expect(screen.getByTestId("run-announcer")).toHaveTextContent("Starting the run…"));
    act(() => useRun.getState().stop());
  });

  it("the skip link moves focus to the main content without changing the route", async () => {
    setFakeApi({});
    const { history } = renderApp("/guide");
    await waitFor(() => expect(document.querySelector("main#main")).not.toBeNull());
    const user = userEvent.setup();
    await user.tab();
    expect(screen.getByTestId("skip-link")).toHaveFocus();
    await user.keyboard("{Enter}");
    expect(document.getElementById("main")).toHaveFocus();
    expect(history.location.pathname).toBe("/guide");
  });
});
