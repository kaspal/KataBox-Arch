import { describe, expect, it, vi } from "vitest";
import { screen, waitFor, within } from "@testing-library/react";
import { useFakeApi } from "@/test/mockApi";
import { renderApp } from "@/test/renderApp";
import { unnamed } from "@/test/a11y";

vi.mock("@/lib/api", async () => (await import("@/test/mockApi")).apiModule);

useFakeApi();

describe("decision guides", () => {
  it("the Guide lists them; answering leads to a Guide entry, and you can start again", async () => {
    const { user, history } = renderApp("/guide");
    const list = await screen.findByTestId("decide-list");
    await user.click(within(list).getByRole("link", { name: "SQL or NoSQL?" }));
    await waitFor(() => expect(history.location.pathname).toBe("/decide/sql-or-nosql"));
    const q = await screen.findByTestId("decide-question");
    expect(q).toHaveTextContent("Do you need joins across your data?");
    await user.click(within(q).getByRole("button", { name: "No" }));
    expect(screen.getByTestId("decide-question")).toHaveTextContent("Must it grow past one machine?");
    await user.click(within(screen.getByTestId("decide-question")).getByRole("button", { name: "Yes" }));
    const result = await screen.findByTestId("decide-result");
    expect(within(result).getByRole("link", { name: /NoSQL Store/ })).toBeInTheDocument();
    expect(result).toHaveTextContent("Partitioning by key scales out.");
    expect(screen.queryByTestId("decide-question")).toBeNull();
    expect(unnamed(screen.getByTestId("decide"))).toEqual([]);
    await user.click(screen.getByRole("button", { name: "Start again" }));
    expect(screen.getByTestId("decide-question")).toHaveTextContent("Do you need joins");
  });

  it("an unknown guide says so", async () => {
    renderApp("/decide/nope");
    expect(await screen.findByText("No such guide")).toBeInTheDocument();
  });
});

describe("the glossary", () => {
  it("lists terms alphabetically, searches, and links to the Guide", async () => {
    const { user } = renderApp("/glossary");
    const list = await screen.findByTestId("glossary");
    const terms = within(list).getAllByRole("term").map((t) => t.textContent);
    expect(terms).toEqual(["Failover", "TTL"]);
    expect(within(list).getByRole("link", { name: "See Cache." })).toBeInTheDocument();
    await user.type(screen.getByRole("searchbox", { name: "Search the glossary" }), "replica");
    expect(within(screen.getByTestId("glossary")).getAllByRole("term").map((t) => t.textContent)).toEqual(["Failover"]);
  });
});
