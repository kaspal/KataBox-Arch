import { describe, expect, it, vi } from "vitest";
import { screen, within } from "@testing-library/react";
import { useFakeApi } from "@/test/mockApi";
import { renderApp } from "@/test/renderApp";
import { unnamed } from "@/test/a11y";

vi.mock("@/lib/api", async () => (await import("@/test/mockApi")).apiModule);

useFakeApi();

describe("the numbers cheat sheet", () => {
  it("groups figures, formats them with units, names their sources and searches", async () => {
    const { user } = renderApp("/numbers");
    expect(await screen.findByRole("heading", { name: "Numbers cheat sheet" })).toBeInTheDocument();
    const sheet = await screen.findByTestId("numbers");
    expect(within(sheet).getByRole("heading", { name: "Memory" })).toBeInTheDocument();
    expect(within(sheet).getByRole("heading", { name: "Network" })).toBeInTheDocument();
    expect(within(sheet).queryByRole("heading", { name: "Sizes" })).toBeNull();
    const values = within(sheet).getAllByTestId("figure-value").map((v) => v.textContent);
    expect(values).toEqual(["100 ns", "1.5 ms", "12,800 req/s"]);
    expect(within(sheet).getByText("Source to be checked (suggested: example.org)")).toBeInTheDocument();
    expect(within(sheet).getByText("The simulator's own figure")).toBeInTheDocument();
    expect(within(sheet).getByText("Source: postgresql.org")).toBeInTheDocument();
    await user.type(screen.getByRole("searchbox", { name: "Search the numbers" }), "zones");
    expect(within(screen.getByTestId("numbers")).getAllByTestId("figure-value")).toHaveLength(1);
    expect(unnamed(document.body)).toEqual([]);
  });

  it("is one click from every brief", async () => {
    renderApp("/problems/url-shortener");
    const link = await screen.findByTestId("numbers-link");
    expect(link).toHaveAttribute("href", expect.stringContaining("/numbers"));
  });
});
