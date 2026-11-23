// M10: the Replays list and a replay's page (what happened, the walkthrough, "Fix it").
import { describe, expect, it, vi } from "vitest";
import { screen } from "@testing-library/react";
import { useFakeApi } from "@/test/mockApi";
import { renderApp } from "@/test/renderApp";
import { unnamed } from "@/test/a11y";

vi.mock("@/lib/api", async () => (await import("@/test/mockApi")).apiModule);

useFakeApi();

describe("Outage replays", () => {
  it("lists the replays, each opening its story", async () => {
    renderApp("/replays");
    expect(await screen.findByRole("heading", { name: "Outage replays" })).toBeInTheDocument();
    const rows = await screen.findAllByTestId("replay-row");
    expect(rows).toHaveLength(1);
    expect(rows[0]).toHaveTextContent("Retry storm");
    expect(rows[0]).toHaveTextContent("Instant retries triple the load.");
    expect(rows[0]).toHaveTextContent("To fix");
    expect(rows[0]).toHaveAttribute("href", expect.stringContaining("/replays/retry-storm"));
    expect(screen.getByTestId("replays-progress")).toHaveTextContent("0 of 1 fixed");
    expect(unnamed(document.body)).toEqual([]);
  });

  it("tells what happened and opens Fix it as the problem outage--<id>", async () => {
    renderApp("/replays/retry-storm");
    expect(await screen.findByTestId("replay-story")).toHaveTextContent("every client retried at once");
    expect(screen.queryByTestId("replay-lesson")).not.toBeInTheDocument();
    expect(screen.getByTestId("replay-fix")).toHaveAttribute("href", expect.stringContaining("/problems/outage--retry-storm"));
  });
});
