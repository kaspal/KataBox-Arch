import { describe, expect, it, vi } from "vitest";
import { screen, waitFor, within } from "@testing-library/react";
import { setFakeApi, useFakeApi } from "@/test/mockApi";
import { renderApp, renderWithProviders } from "@/test/renderApp";
import { unnamed } from "@/test/a11y";
import { Markdown } from "@/components/Markdown";

vi.mock("@/lib/api", async () => (await import("@/test/mockApi")).apiModule);

const fake = useFakeApi();

describe("problem packs", () => {
  it("previews a pack before installing it, then lists its problems with a badge and a filter", async () => {
    setFakeApi({ pack: "ok" });
    const { user } = renderApp("/problems");
    await screen.findByTestId("problem-row");
    expect(screen.queryByRole("group", { name: "Source" })).not.toBeInTheDocument();
    await user.click(screen.getByTestId("import-pack"));
    const preview = await screen.findByTestId("pack-preview");
    // The unsigned warning names the author as the pack states it; the hash is shown.
    expect(within(preview).getByTestId("pack-unsigned")).toHaveTextContent("Unsigned pack, from Ana (as stated in the pack)");
    expect(within(preview).getByTestId("pack-sha")).toHaveTextContent(/^c0ffee0+$/);
    expect(preview).toHaveTextContent("1 problem");
    expect(preview).toHaveTextContent("Demo drill");
    expect(within(preview).getByTestId("pack-change")).toHaveTextContent("New: it will be added to your problems.");
    expect(unnamed(document.body)).toEqual([]);
    // Nothing is installed until the user confirms.
    expect(fake().api.applyPack).not.toHaveBeenCalled();
    await user.click(screen.getByTestId("pack-install"));
    expect(fake().api.applyPack).toHaveBeenCalledWith(7);
    await waitFor(() => expect(screen.getAllByTestId("problem-row")).toHaveLength(2));
    expect(screen.getByTestId("pack-badge")).toHaveTextContent("From the pack Drills");
    const source = screen.getByRole("group", { name: "Source" });
    await user.click(within(source).getByRole("button", { name: "Packs" }));
    expect(screen.getAllByTestId("problem-row")).toHaveLength(1);
    expect(screen.getByTestId("problem-row")).toHaveTextContent("Demo drill");
    await user.click(within(source).getByRole("button", { name: "Built-in" }));
    expect(screen.getByTestId("problem-row")).toHaveTextContent("URL Shortener");
  });

  it("cancelling the preview drops the staged pack", async () => {
    setFakeApi({ pack: "ok" });
    const { user } = renderApp("/problems");
    await user.click(await screen.findByTestId("import-pack"));
    await screen.findByTestId("pack-preview");
    await user.click(screen.getByRole("button", { name: "Cancel" }));
    await waitFor(() => expect(fake().api.cancelPack).toHaveBeenCalledWith(7));
    expect(fake().api.applyPack).not.toHaveBeenCalled();
  });

  it("a refused pack says why, as plain text", async () => {
    setFakeApi({ pack: "refused" });
    const { user } = renderApp("/problems");
    await user.click(await screen.findByTestId("import-pack"));
    const box = await screen.findByTestId("pack-error");
    expect(box).toHaveTextContent("This pack failed its safety and content checks");
    expect(within(box).getByRole("list", { name: "Why" })).toHaveTextContent("a <script> tag is not allowed in pack text");
    expect(box.querySelector("script")).toBeNull();
  });

  it("Settings lists installed packs, uninstalls one and sets whether packs are recommended", async () => {
    setFakeApi({ pack: "ok" });
    vi.spyOn(window, "confirm").mockReturnValue(true);
    const { user } = renderApp("/settings");
    const section = await screen.findByTestId("packs-settings");
    expect(section).toHaveTextContent("No packs installed.");
    await user.click(within(section).getByTestId("import-pack"));
    await user.click(await screen.findByTestId("pack-install"));
    const row = await within(section).findByTestId("installed-pack");
    expect(row).toHaveTextContent("Drills by Ana · version 1.0.0 · 1 problems");
    await user.click(within(row).getByRole("button", { name: "Uninstall Drills" }));
    await waitFor(() => expect(fake().api.uninstallPack).toHaveBeenCalledWith("drills"));
    expect(await within(section).findByText("No packs installed.")).toBeInTheDocument();
    await user.click(within(section).getByRole("checkbox", { name: "Recommend problems from packs on Home" }));
    await waitFor(() => expect(fake().api.setSetting).toHaveBeenCalledWith("packs.recommend", true));
  });

  it("pack text shows images as their alt text and links as plain text", async () => {
    renderWithProviders(<Markdown trust="pack">{"![a diagram](http://example.invalid/x.png) and [a link](http://example.invalid)"}</Markdown>);
    expect(await screen.findByTestId("inert-image")).toHaveTextContent("[a diagram]");
    expect(document.querySelector("img")).toBeNull();
    expect(document.querySelector("a")).toBeNull();
  });
});
