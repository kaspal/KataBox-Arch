import { describe, expect, it, vi } from "vitest";
import { screen, waitFor, within } from "@testing-library/react";
import { setFakeApi, useFakeApi } from "@/test/mockApi";
import { renderApp } from "@/test/renderApp";
import { unnamed } from "@/test/a11y";

vi.mock("@/lib/api", async () => (await import("@/test/mockApi")).apiModule);

const fake = useFakeApi();

describe("learning paths", () => {
  it("show each path's items with progress and links, and follow one", async () => {
    const { user } = renderApp("/paths");
    const paths = await screen.findAllByTestId("path");
    expect(paths).toHaveLength(2);
    const first = paths[0]!;
    expect(within(first).getByRole("heading", { name: "Foundations" })).toBeInTheDocument();
    const items = within(first).getAllByTestId("path-item");
    expect(items.map((i) => i.textContent)).toEqual(["1.Read: Cache", "2.Solve: URL Shortener", "3.Find the flaw: A cache that never forgets"]);
    expect(items[1]).toHaveAttribute("href", expect.stringContaining("/problems/url-shortener"));
    expect(items[2]).toHaveAttribute("href", expect.stringContaining("/problems/flaw--cache-no-ttl"));
    expect(within(first).getByTestId("path-progress")).toHaveTextContent("0 of 3 done");
    expect(within(first).getByRole("progressbar", { name: "Progress on Foundations" })).toHaveAttribute("aria-valuenow", "0");
    await user.click(within(first).getByRole("button", { name: "Follow this path" }));
    expect(fake().api.setSetting).toHaveBeenCalledWith("paths.active", "foundations");
    expect(await within(first).findByTestId("path-following")).toHaveTextContent("Following");
    expect(unnamed(document.body)).toEqual([]);
  });

  it("Home recommends the followed path's next unfinished item", async () => {
    setFakeApi({ settings: { "paths.active": "foundations", "tour.offered": true } });
    renderApp("/");
    const card = await screen.findByTestId("home-path-next");
    expect(card).toHaveTextContent("Read: Cache");
    expect(card).toHaveTextContent("Foundations · 0 of 3 done");
    await waitFor(() => expect(screen.queryByTestId("home-recommended")).toBeNull());
  });
});
