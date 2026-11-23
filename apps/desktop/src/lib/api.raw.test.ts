import { describe, expect, it, vi } from "vitest";

// The real `lib/api` with Tauri's `invoke` stood in for: the two raw commands (M7b.2) are
// called by hand, so their shape is checked here.
const calls = vi.hoisted(() => [] as unknown[][]);
vi.mock("@tauri-apps/api/core", async (orig) => ({
  ...(await orig<typeof import("@tauri-apps/api/core")>()),
  invoke: vi.fn(async (...args: unknown[]) => {
    calls.push(args);
    return args[0] === "run_frames" ? new Uint8Array([1, 2, 3]).buffer : "C:/out/run.webm";
  }),
}));

import { api } from "@/lib/api";

describe("raw-body commands", () => {
  it("saveExport sends the bytes as the body, the format and the encoded name as headers", async () => {
    const bytes = new Uint8Array([26, 69, 223, 163]);
    expect(await api.saveExport("webm", bytes, "Café & co.webm")).toBe("C:/out/run.webm");
    const [cmd, body, opts] = calls.at(-1)!;
    expect(cmd).toBe("save_export");
    expect(body).toBe(bytes);
    expect(opts).toEqual({ headers: { "ka-format": "webm", "ka-name": "Caf%C3%A9%20%26%20co.webm" } });
  });

  it("runFrames sends the request and hands back the raw bytes", async () => {
    const req = { source: { type: "submission" as const, submission_id: 7, test_id: "viral" }, from_s: 20, seconds: 10, fps: 30 as const };
    const buf = await api.runFrames(req);
    expect(new Uint8Array(buf)).toEqual(new Uint8Array([1, 2, 3]));
    expect(calls.at(-1)).toEqual(["run_frames", { request: req }]);
  });
});
