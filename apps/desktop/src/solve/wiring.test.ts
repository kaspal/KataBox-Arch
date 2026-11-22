import { describe, expect, it } from "vitest";
import type { ComponentKind, Protocol, WiringReason } from "@/bindings";
import fixture from "@/test/fixtures/registry.json";
import { REGISTRY } from "@/test/fakeApi";
import { PROTOCOLS, checkWire, makeWiring, protocolStyle } from "@/solve/wiring";

type Cell = { from: ComponentKind; to: ComponentKind; protocols: Protocol[]; reason: WiringReason | null };
const matrix = fixture.matrix as Cell[];
const wiring = makeWiring(REGISTRY);

describe("wiring parity with Rust", () => {
  it("covers every kind pair", () => {
    expect(matrix).toHaveLength(REGISTRY.kinds.length ** 2);
  });

  it("the TypeScript evaluator reproduces Rust's kind × kind matrix exactly", () => {
    const mismatches = matrix.filter((cell) => {
      const protocols = wiring.allowed(cell.from, cell.to);
      const reason = protocols.length === 0 ? wiring.reason(cell.from, cell.to) : null;
      return JSON.stringify(protocols) !== JSON.stringify(cell.protocols) || reason !== cell.reason;
    });
    expect(mismatches).toEqual([]);
  });

  it("PROTOCOLS follows Rust's order", () => {
    // Every allowed list in the matrix is sorted by it.
    for (const cell of matrix) {
      const sorted = [...cell.protocols].sort((a, b) => PROTOCOLS.indexOf(a) - PROTOCOLS.indexOf(b));
      expect(cell.protocols).toEqual(sorted);
    }
  });
});

describe("checkWire", () => {
  const nodes = [
    { id: "c", kind: "client" as const },
    { id: "a", kind: "app" as const },
    { id: "db", kind: "sql" as const },
  ];

  it("blocks self-loops and refuses with the table's reason", () => {
    expect(checkWire(wiring, nodes, [], "a", "a")).toEqual({ ok: false, reason: "self_loop" });
    expect(checkWire(wiring, nodes, [], "c", "db")).toEqual({ ok: false, reason: "client_to_data" });
  });

  it("offers only protocols not already used for the pair", () => {
    expect(checkWire(wiring, nodes, [], "c", "a")).toEqual({ ok: true, protocols: ["http", "grpc", "ws"] });
    const used = [{ from: "c", to: "a", protocol: "http" as const }];
    expect(checkWire(wiring, nodes, used, "c", "a")).toEqual({ ok: true, protocols: ["grpc", "ws"] });
    const query = [{ from: "a", to: "db", protocol: "query" as const }];
    expect(checkWire(wiring, nodes, query, "a", "db")).toEqual({ ok: false, reason: "duplicate_edge" });
  });

  it("draws async dashed and replication dotted", () => {
    expect(protocolStyle("http")).toBe("solid");
    expect(protocolStyle("publish")).toBe("dashed");
    expect(protocolStyle("consume")).toBe("dashed");
    expect(protocolStyle("replicate")).toBe("dotted");
  });
});
