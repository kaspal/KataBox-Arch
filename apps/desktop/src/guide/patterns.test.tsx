import { describe, expect, it, vi } from "vitest";
import { act, screen, waitFor, within } from "@testing-library/react";
import { useFakeApi } from "@/test/mockApi";
import { renderApp } from "@/test/renderApp";
import { unnamed } from "@/test/a11y";
import { undo, useEditor } from "@/solve/store";

vi.mock("@/lib/api", async () => (await import("@/test/mockApi")).apiModule);

const fake = useFakeApi();

describe("the pattern library", () => {
  it("lists patterns and opens one", async () => {
    const { user, history } = renderApp("/patterns");
    const card = await screen.findByTestId("pattern-card");
    expect(card).toHaveTextContent("Cache-aside");
    expect(screen.getByTestId("patterns-progress")).toHaveTextContent("0 of 1 read");
    await user.click(card);
    await waitFor(() => expect(history.location.pathname).toBe("/patterns/cache-aside"));
    const page = await screen.findByTestId("pattern");
    expect(within(page).getByRole("heading", { level: 1, name: "Cache-aside" })).toBeInTheDocument();
    expect(within(page).getByText("cache").tagName).toBe("STRONG");
    expect(within(page).getByTestId("pattern-sim")).toHaveTextContent("Fully: hits, misses and TTL are simulated.");
    expect(within(page).getByRole("link", { name: "Cache" })).toBeInTheDocument();
    expect(unnamed(page)).toEqual([]);
    await user.click(within(page).getByRole("button", { name: "Mark as read" }));
    expect(fake().api.setPatternRead).toHaveBeenCalledWith("cache-aside", true);
  });

  it("Insert adds the template with fresh ids as one undo step", async () => {
    const { user, history } = renderApp("/patterns/cache-aside");
    const page = await screen.findByTestId("pattern");
    const before = useEditor.getState().doc.nodes.length;
    await user.click(within(page).getByRole("button", { name: "Insert into the Sandbox" }));
    await waitFor(() => expect(history.location.pathname).toBe("/sandbox"));
    await waitFor(() => expect(useEditor.getState().doc.nodes).toHaveLength(before + 3));
    const doc = useEditor.getState().doc;
    expect(doc.nodes.map((n) => n.id)).not.toContain("cache");
    expect(doc.edges).toHaveLength(2);
    // The edges point at the new cards.
    const ids = new Set(doc.nodes.map((n) => n.id));
    expect(doc.edges.every((e) => ids.has(e.from) && ids.has(e.to))).toBe(true);
    expect(useEditor.getState().selection).toHaveLength(3);
    act(() => undo());
    expect(useEditor.getState().doc.nodes).toHaveLength(before);
  });

  it("Ctrl+K finds patterns", async () => {
    const { user, history } = renderApp("/");
    await screen.findByTestId("home-samples");
    await user.keyboard("{Control>}k{/Control}");
    await user.type(await screen.findByRole("combobox"), "pattern cache");
    await user.click(await screen.findByRole("option", { name: "Pattern: Cache-aside" }));
    await waitFor(() => expect(history.location.pathname).toBe("/patterns/cache-aside"));
  });
});
