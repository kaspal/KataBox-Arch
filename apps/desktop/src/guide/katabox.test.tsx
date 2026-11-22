import { describe, expect, it, vi } from "vitest";
import { screen, within } from "@testing-library/react";
import { useFakeApi } from "@/test/mockApi";
import { renderApp } from "@/test/renderApp";

vi.mock("@/lib/api", async () => (await import("@/test/mockApi")).apiModule);

const fake = useFakeApi();

describe("Related in KataBox", () => {
  it("lists the KataBox topics and problems behind a page, as titles only", async () => {
    renderApp("/problems/url-shortener");
    const box = await screen.findByTestId("katabox-links");
    expect(within(box).getByRole("heading", { name: "Related in KataBox" })).toBeInTheDocument();
    expect(box).toHaveTextContent("LRU cacheLearn topic");
    expect(box).toHaveTextContent("League RankProblem");
    // No deep links into another app.
    expect(box.querySelector("a, button")).toBeNull();
    expect(fake().api.kataboxLinks).toHaveBeenCalledWith("problem:url-shortener");
  });
});
