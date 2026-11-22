import { describe, expect, it, vi } from "vitest";
import { screen, within } from "@testing-library/react";
import { useFakeApi } from "@/test/mockApi";
import { renderApp } from "@/test/renderApp";
import { unnamed } from "@/test/a11y";

vi.mock("@/lib/api", async () => (await import("@/test/mockApi")).apiModule);

useFakeApi();

describe("Find the flaw", () => {
  it("lists the drills with their kinds and status, each opening as a problem", async () => {
    renderApp("/flaws");
    expect(await screen.findByRole("heading", { name: "Find the flaw" })).toBeInTheDocument();
    const rows = await screen.findAllByTestId("flaw-row");
    expect(rows).toHaveLength(2);
    expect(rows[0]).toHaveTextContent("A cache that never forgets");
    expect(rows[0]).toHaveTextContent("Cache");
    expect(rows[0]).toHaveTextContent("To fix");
    expect(rows[1]).toHaveTextContent("Fixed");
    expect(rows[0]).toHaveAttribute("href", expect.stringContaining("/problems/flaw--cache-no-ttl"));
    expect(screen.getByTestId("flaws-progress")).toHaveTextContent("1 of 2 fixed");
    expect(within(screen.getByTestId("flaw-list")).getAllByRole("link")).toHaveLength(2);
    expect(unnamed(document.body)).toEqual([]);
  });
});
