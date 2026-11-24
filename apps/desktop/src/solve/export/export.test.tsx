import { describe, expect, it, vi } from "vitest";
import { screen, waitFor } from "@testing-library/react";
import { useFakeApi } from "@/test/mockApi";
import { renderApp } from "@/test/renderApp";

vi.mock("@/lib/api", async () => (await import("@/test/mockApi")).apiModule);

const fake = useFakeApi();

describe("the Export menu", () => {
  it("copies the design as Mermaid with names in the user's language, and saves it through Rust", async () => {
    const { user } = renderApp("/sandbox");
    await user.click(await screen.findByTestId("export-menu"));
    await user.click(await screen.findByRole("menuitem", { name: "Copy as Mermaid" }));
    // user-event stands in for the clipboard: read back what was written.
    await waitFor(async () => expect(await navigator.clipboard.readText()).toBe("flowchart LR\n"));
    const [, names] = vi.mocked(fake().api.mermaidText).mock.calls[0]!;
    expect(names["kind.app"]).toBe("App Server");
    expect(names["protocol.http"]).toBe("HTTP");
    expect(names["group.region"]).toBe("Region");
    expect(await screen.findByText("Mermaid diagram copied.")).toBeInTheDocument();
    await user.click(screen.getByTestId("export-menu"));
    await user.click(await screen.findByRole("menuitem", { name: "Save as Mermaid…" }));
    await waitFor(() => expect(fake().api.saveMermaid).toHaveBeenCalled());
    expect(vi.mocked(fake().api.saveMermaid).mock.calls[0]![2]).toMatch(/\.mmd$/);
  });

  it("doesn't offer a share link until the web viewer that opens it ships (H6)", async () => {
    const { user } = renderApp("/sandbox");
    await user.click(await screen.findByTestId("export-menu"));
    expect(await screen.findByRole("menuitem", { name: "Copy as Mermaid" })).toBeInTheDocument();
    expect(screen.queryByTestId("export-share-link")).not.toBeInTheDocument();
  });

  it("saves the design as an SVG image through Rust", async () => {
    const { user } = renderApp("/sandbox");
    await user.click(await screen.findByTestId("export-menu"));
    await user.click(await screen.findByRole("menuitem", { name: "Save as SVG image…" }));
    await waitFor(() => expect(fake().api.saveExport).toHaveBeenCalled());
    const [format, bytes, name] = vi.mocked(fake().api.saveExport).mock.calls[0]!;
    expect(format).toBe("svg");
    expect(new TextDecoder().decode(bytes)).toMatch(/^<svg xmlns/);
    expect(name).toMatch(/\.svg$/);
  });
});