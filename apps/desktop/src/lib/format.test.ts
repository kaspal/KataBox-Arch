import { describe, expect, it } from "vitest";
import i18n from "i18next";
import { formatMs, percentileLabel } from "@/lib/format";

describe("latency formats (review F2)", () => {
  it("shows milliseconds below a second and seconds above", () => {
    expect(formatMs("en", 31.24)).toBe("31.2 ms");
    expect(formatMs("en", 786)).toBe("786 ms");
    expect(formatMs("en", 25_144.8)).toBe("25.1 s");
    expect(formatMs("en", 120_000)).toBe("120 s");
    expect(formatMs("es", 25_144.8)).toBe("25,1 s");
  });

  it("labels completion-judged limits as completion", () => {
    const t = i18n.getFixedT("en");
    expect(percentileLabel(t, "p95", "completion")).toBe("p95 completion");
    expect(percentileLabel(t, "p95", "response")).toBe("p95 latency");
    expect(percentileLabel(t, "p99", "mixed")).toBe("p99 (answers and completions)");
    expect(percentileLabel(i18n.getFixedT("es"), "p95", "completion")).toBe("Finalización p95");
  });
});
