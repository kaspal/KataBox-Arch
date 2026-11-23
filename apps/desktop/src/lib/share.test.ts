// M13 share links: the fragment round-trips, and oversized designs are refused.
import { describe, expect, it } from "vitest";
import { SHARE_BASE, SHARE_LIMIT_BYTES, ShareTooLarge, readShareLink, shareLink } from "@/lib/share";

describe("share links", () => {
  it("carry a design's JSON compressed in the fragment", async () => {
    const json = JSON.stringify({ schema_version: 4, id: "d", name: "Café ☕", nodes: Array.from({ length: 20 }, (_, i) => ({ id: `n${i}` })) });
    const link = await shareLink(json);
    expect(link.startsWith(`${SHARE_BASE}#v1.`)).toBe(true);
    expect(link.slice(link.indexOf("#"))).not.toMatch(/[+/=]/);
    expect(link.length).toBeLessThan(json.length + SHARE_BASE.length + 10);
    expect(await readShareLink(link)).toBe(json);
  });

  it("refuse a design too big for a link", async () => {
    // Random-looking text barely compresses.
    let s = "";
    let x = 7;
    for (let i = 0; i < SHARE_LIMIT_BYTES * 2; i++) {
      // xorshift32: 32-bit integer arithmetic, so the sequence never degenerates.
      x ^= x << 13;
      x ^= x >>> 17;
      x ^= x << 5;
      s += String.fromCharCode(33 + ((x >>> 0) % 90));
    }
    await expect(shareLink(s)).rejects.toBeInstanceOf(ShareTooLarge);
  });
});
