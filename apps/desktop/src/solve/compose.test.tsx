import { describe, expect, it, vi } from "vitest";
import { act, screen, waitFor, within } from "@testing-library/react";
import type { ComposeImport } from "@/bindings";
import { setFakeApi, useFakeApi } from "@/test/mockApi";
import { renderApp } from "@/test/renderApp";
import { unnamed } from "@/test/a11y";
import { isDirty, useEditor } from "@/solve/store";

vi.mock("@/lib/api", async () => (await import("@/test/mockApi")).apiModule);

const fake = useFakeApi();

const IMPORTED: ComposeImport = {
  design: {
    schema_version: 3,
    id: "compose-1",
    name: "docker-compose",
    problem_id: null,
    nodes: [
      { id: "web", kind: "lb", flavour: "nginx", label: "web", position: { x: 0, y: 0 }, props: {}, group_id: null },
      { id: "api", kind: "app", flavour: "go", label: "api", position: { x: 304, y: 0 }, props: { instances: 3 }, group_id: null },
      { id: "db", kind: "sql", flavour: "postgresql", label: "db", position: { x: 608, y: 0 }, props: {}, group_id: null },
    ],
    edges: [
      { id: "e1", from: "web", to: "api", protocol: "http" },
      { id: "e2", from: "api", to: "db", protocol: "query" },
    ],
    groups: [],
    notes: [],
    viewport: { x: 0, y: 0, zoom: 1 },
  },
  mapped: [
    { service: "web", image: "nginx:1.27", kind: "lb", flavour: "nginx" },
    { service: "db", image: "postgres:16", kind: "sql", flavour: "postgresql" },
  ],
  guessed: [{ service: "api", image: null, kind: "app", flavour: "go" }],
  clamped: ["api"],
  skipped_edges: [
    { from: "db", to: "cache", reason: { type: "not_allowed", wiring: "data_calls_out" } },
    { from: "api", to: "ghost", reason: { type: "unknown_service" } },
  ],
};

describe("docker-compose import", () => {
  it("opens the file as a new unsaved design and summarises what it guessed", async () => {
    setFakeApi({ composeImport: IMPORTED });
    const { user } = renderApp("/sandbox");
    await user.click(await screen.findByRole("button", { name: "Import docker-compose…" }));
    const summary = await screen.findByTestId("compose-summary");
    expect(fake().api.importComposeDialog).toHaveBeenCalled();
    expect(summary).toHaveTextContent("3 cards and 2 connections");
    expect(within(summary).getByTestId("compose-mapped")).toHaveTextContent("2 services recognised by their images");
    expect(within(summary).getByTestId("compose-mapped")).toHaveTextContent("PostgreSQL · SQL Database");
    expect(within(summary).getByTestId("compose-guessed")).toHaveTextContent("1 service drawn as an App Server");
    expect(within(summary).getByTestId("compose-guessed")).toHaveTextContent("built from source");
    expect(within(summary).getByTestId("compose-clamped")).toHaveTextContent("api");
    const skipped = within(summary).getByTestId("compose-skipped");
    expect(skipped).toHaveTextContent("2 dependencies not drawn");
    expect(skipped).toHaveTextContent("db → cache");
    expect(skipped).toHaveTextContent("No service with that name is in the file.");
    expect(unnamed(summary)).toEqual([]);
    // The design is in the editor, unsaved, with no file yet.
    expect(useEditor.getState().doc.nodes).toHaveLength(3);
    expect(useEditor.getState().path).toBeNull();
    expect(isDirty()).toBe(true);
    await user.click(within(summary).getByRole("button", { name: "Done" }));
    await waitFor(() => expect(screen.queryByTestId("compose-summary")).toBeNull());
  });

  it("asks about unsaved changes first, and does nothing when cancelled", async () => {
    setFakeApi({ composeImport: IMPORTED });
    const { user } = renderApp("/sandbox");
    await screen.findByRole("button", { name: "Import docker-compose…" });
    act(() => useEditor.getState().rename("Edited"));
    await user.click(screen.getByRole("button", { name: "Import docker-compose…" }));
    const ask = await screen.findByTestId("unsaved-dialog");
    await user.click(within(ask).getByRole("button", { name: "Cancel" }));
    expect(fake().api.importComposeDialog).not.toHaveBeenCalled();
    expect(useEditor.getState().doc.name).toBe("Edited");
  });

  it("is in the command palette", async () => {
    const { user } = renderApp("/");
    await user.keyboard("{Control>}k{/Control}");
    expect(await screen.findByRole("option", { name: "Import docker-compose…" })).toBeInTheDocument();
  });
});
