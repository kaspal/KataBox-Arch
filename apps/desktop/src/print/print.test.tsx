import { describe, expect, it, vi } from "vitest";
import { screen, waitFor, within } from "@testing-library/react";
import { setFakeApi, useFakeApi } from "@/test/mockApi";
import { renderApp } from "@/test/renderApp";
import { printHtml } from "@/print/printing";

vi.mock("@/lib/api", async () => (await import("@/test/mockApi")).apiModule);

const fake = useFakeApi();

describe("the printable Guide (M7b.3)", () => {
  it("prints an entry to PDF through Rust once the page is ready, then goes back", async () => {
    const { user, history } = renderApp("/guide/cache");
    await screen.findByTestId("guide-entry");
    await user.click(screen.getByTestId("print-menu"));
    await user.click(await screen.findByRole("menuitem", { name: "Save as PDF…" }));
    // Rust keeps the path; the UI only has a token, and moves to the print page.
    await waitFor(() => expect(fake().api.printPdf).toHaveBeenCalledWith(7));
    expect(vi.mocked(fake().api.pickPdfPath).mock.calls[0]![0]).toBe("Cache.pdf");
    await waitFor(() => expect(history.location.pathname).toBe("/guide/cache"));
    expect(await screen.findByTestId("notice")).toHaveTextContent("PDF saved.");
  });

  it("cancelled Save dialog: nothing happens", async () => {
    setFakeApi({ pdfToken: null });
    const { user, history } = renderApp("/guide/cache");
    await screen.findByTestId("guide-entry");
    await user.click(screen.getByTestId("print-menu"));
    await user.click(await screen.findByRole("menuitem", { name: "Save as PDF…" }));
    await waitFor(() => expect(fake().api.pickPdfPath).toHaveBeenCalled());
    expect(fake().api.printPdf).not.toHaveBeenCalled();
    expect(history.location.pathname).toBe("/guide/cache");
  });

  it("when the PDF fails, goes back and offers the print-ready HTML instead", async () => {
    setFakeApi({ pdfError: { code: "pdf_unsupported", message: "no" } });
    const { user, history } = renderApp("/guide/cache");
    await screen.findByTestId("guide-entry");
    await user.click(screen.getByTestId("print-menu"));
    await user.click(await screen.findByRole("menuitem", { name: "Save as PDF…" }));
    const offer = await screen.findByTestId("print-fallback");
    expect(history.location.pathname).toBe("/guide/cache");
    expect(offer).toHaveTextContent("This system's web view can't print to PDF.");
    await user.click(within(offer).getByRole("button", { name: "Save print-ready HTML…" }));
    await waitFor(() => expect(fake().api.saveExport).toHaveBeenCalled());
    const [format, bytes, name] = vi.mocked(fake().api.saveExport).mock.calls[0]!;
    expect(format).toBe("html");
    expect(name).toBe("Cache.html");
    const html = new TextDecoder().decode(bytes);
    expect(html).toMatch(/^<!doctype html>/);
    expect(html).toContain("Cache");
    expect(html).not.toMatch(/<script|<button|\son\w+=/i);
    await waitFor(() => expect(history.location.pathname).toBe("/guide/cache"));
  });

  it("lays out the whole Guide, then the glossary, without the app around it", async () => {
    renderApp("/print?what=guide");
    const root = await screen.findByTestId("print-root");
    await waitFor(() => expect(root).toHaveAttribute("data-print-ready", "1"));
    expect(within(root).getAllByTestId("print-entry")).toHaveLength(2);
    expect(within(root).getByTestId("print-glossary")).toBeInTheDocument();
    // No rail, no quiz, no overlay when only viewing.
    expect(screen.queryByRole("navigation", { name: /main/i })).toBeNull();
    expect(screen.queryByTestId("quiz")).toBeNull();
    expect(screen.queryByTestId("print-overlay")).toBeNull();
    // Glossary terms are plain text on paper, not links.
    expect(within(root).queryAllByTestId("glossary-term")).toHaveLength(0);
  });

  it("prints a pattern with its template's picture", async () => {
    renderApp("/print?what=pattern:cache-aside");
    const root = await screen.findByTestId("print-root");
    await waitFor(() => expect(root).toHaveAttribute("data-print-ready", "1"));
    expect(within(root).getByTestId("print-pattern").querySelector("svg")).not.toBeNull();
  });
});

describe("the print-ready HTML", () => {
  it("keeps the content and the CSS, and drops anything that runs", () => {
    const root = document.createElement("div");
    root.innerHTML = `<h1 onclick="steal()">Cache</h1><a href="javascript:alert(1)">x</a><script>alert(1)</script><button>Cancel</button><div data-print-overlay>Preparing</div><p>Body</p>`;
    const sheet = { cssRules: [{ cssText: "@font-face { font-family: X; src: url(/x.woff2); }" }, { cssText: "h1 { color: red; }" }, { cssText: "p { margin: 0 } </style><script>" }] } as unknown as CSSStyleSheet;
    const html = printHtml(root, "Cache <Guide>", "es", [sheet]);
    expect(html).toContain('<html lang="es">');
    expect(html).toContain("<title>Cache &lt;Guide&gt;</title>");
    expect(html).toContain("<h1>Cache</h1>");
    expect(html).toContain("<p>Body</p>");
    expect(html).toContain("h1 { color: red; }");
    expect(html).not.toContain("@font-face");
    expect(html).not.toMatch(/<script|<button|onclick|javascript:|Preparing/i);
    expect(html).not.toMatch(/<\/style><script/);
  });
});
