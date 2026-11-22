import { describe, expect, it, vi } from "vitest";
import { screen, waitFor, within } from "@testing-library/react";
import { useFakeApi } from "@/test/mockApi";
import { renderApp } from "@/test/renderApp";
import { unnamed } from "@/test/a11y";
import { fillText } from "@/practice/EstimateDrill";

vi.mock("@/lib/api", async () => (await import("@/test/mockApi")).apiModule);

const fake = useFakeApi();

describe("estimation drills", () => {
  it("fill a text's inputs and answer in the reader's number format", () => {
    expect(fillText("{a} and {b} make {answer} {nope}", { a: 12_000_000, b: 0.25 }, "en", 3472.2)).toBe("12,000,000 and 0.25 make 3,472 {nope}");
    expect(fillText("{a}", { a: 2.55 }, "es")).toBe("2,6");
  });

  it("the brief folds its drills; checking one shows the verdict, the answer and the worked solution", async () => {
    const { user } = renderApp("/problems/url-shortener");
    const section = await screen.findByTestId("estimate-first");
    expect(within(section).getByTestId("estimate-count")).toHaveTextContent("0 of 2 estimated");
    expect(within(section).queryByTestId("estimate-drill")).toBeNull();
    await user.click(within(section).getByRole("button", { name: /Estimate first/ }));
    const drill = within(section).getAllByTestId("estimate-drill")[0]!;
    expect(drill).toHaveTextContent("864,000,000 redirects a day");
    const box = within(drill).getByRole("textbox", { name: "Your answer" });
    await user.type(box, "50000");
    await user.click(within(drill).getByRole("button", { name: "Check" }));
    expect(await within(drill).findByTestId("estimate-result")).toHaveTextContent("Too high, but the right order of magnitude.");
    expect(within(drill).getByTestId("estimate-result")).toHaveTextContent("The answer: 10,000 req/s");
    await user.click(within(drill).getByRole("button", { name: "Try again" }));
    await user.type(within(drill).getByRole("textbox", { name: "Your answer" }), "12,000");
    await user.click(within(drill).getByRole("button", { name: "Check" }));
    const result = await within(drill).findByTestId("estimate-result");
    expect(result).toHaveTextContent("Close enough");
    expect(result).toHaveTextContent("864,000,000 ÷ 86,400 = 10,000");
    await waitFor(() => expect(within(section).getByTestId("estimate-count")).toHaveTextContent("1 of 2 estimated"));
    expect(fake().api.checkEstimate).toHaveBeenLastCalledWith("url-shortener", "redirect-qps", 12000);
  });

  it("the trainer page lists a problem's drills and counts progress", async () => {
    renderApp("/estimate");
    expect(await screen.findByRole("heading", { name: "Estimation drills" })).toBeInTheDocument();
    expect(await screen.findAllByTestId("estimate-drill")).toHaveLength(2);
    expect(screen.getByTestId("estimate-progress")).toHaveTextContent("0 of 2 drills done");
    expect(unnamed(document.body)).toEqual([]);
  });
});
