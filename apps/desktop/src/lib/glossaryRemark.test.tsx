import { describe, expect, it, vi } from "vitest";
import { screen, within } from "@testing-library/react";
import { compileNeedles, splitText } from "@/lib/glossaryRemark";
import { Markdown } from "@/components/Markdown";
import { useFakeApi } from "@/test/mockApi";
import { renderWithProviders } from "@/test/renderApp";

vi.mock("@/lib/api", async () => (await import("@/test/mockApi")).apiModule);
useFakeApi();

const needles = compileNeedles([
  { id: "ttl", text: "TTL" },
  { id: "hit-rate", text: "cache hit rate" },
  { id: "cache", text: "cache" },
  { id: "p95", text: "p95" },
]);

const terms = (nodes: ReturnType<typeof splitText>) =>
  nodes.filter((n) => n.type === "glossaryTerm").map((n) => [n.data?.hProperties?.dataTerm, n.children?.[0].value]);

describe("the glossary remark plugin", () => {
  it("marks the first use of each term, whole words only, ignoring case", () => {
    const used = new Set<string>();
    const out = splitText("A ttl of 30 s and another TTL; the p95 rises. TTLs aside, p950 is not p95.", needles, used);
    expect(terms(out)).toEqual([
      ["ttl", "ttl"],
      ["p95", "p95"],
    ]);
    // The text is preserved around the marks.
    expect(out.map((n) => n.value ?? n.children?.[0].value).join("")).toBe(
      "A ttl of 30 s and another TTL; the p95 rises. TTLs aside, p950 is not p95.",
    );
  });

  it("prefers the longest term at the same spot", () => {
    const out = splitText("The cache hit rate falls when the cache is cold.", needles, new Set());
    expect(terms(out)).toEqual([
      ["hit-rate", "cache hit rate"],
      ["cache", "cache"],
    ]);
  });

  it("handles accented letters as part of words", () => {
    const es = compileNeedles([{ id: "replica", text: "réplica" }]);
    expect(terms(splitText("las réplicas y una réplica", es, new Set()))).toEqual([["replica", "réplica"]]);
  });
});

describe("glossary terms in content prose", () => {
  it("link the first use to the glossary, but never inside headings or code", async () => {
    renderWithProviders(<Markdown>{"## TTL\n\nSet a `TTL` here. A short TTL keeps data fresh; the TTL again.\n\nFailover takes time."}</Markdown>);
    const links = await screen.findAllByTestId("glossary-term");
    expect(links.map((l) => l.textContent)).toEqual(["TTL", "Failover"]);
    expect(links[0].getAttribute("href")).toContain("term=ttl");
    expect(within(screen.getByRole("heading")).queryByTestId("glossary-term")).toBeNull();
  });

  it("can be turned off", async () => {
    renderWithProviders(<Markdown glossary={false}>{"A short TTL."}</Markdown>);
    await screen.findByText("A short TTL.");
    expect(screen.queryByTestId("glossary-term")).toBeNull();
  });
});
