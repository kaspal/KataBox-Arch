// M13 share links: a design travels in the link's fragment, compressed. Rust supplies the
// canonical .kbarch JSON (validated, the same bytes Save writes); this module compresses it with
// deflate-raw and base64url-encodes it as `<SHARE_BASE>#v1.<payload>`. The fragment never
// reaches a server, and the web viewer that opens it is a later step (deferred; see the M8–M14
// report). Links over SHARE_LIMIT_BYTES are refused: share the file instead.

/** Where the web viewer will live (a maintainer step turns on hosting; see maintainer-steps). */
export const SHARE_BASE = "https://kaspal.github.io/KataBox-Arch/";
export const SHARE_PREFIX = "#v1.";
export const SHARE_LIMIT_BYTES = 24 * 1024;

export class ShareTooLarge extends Error {
  constructor(readonly bytes: number) {
    super(`share link payload is ${bytes} bytes (limit ${SHARE_LIMIT_BYTES})`);
  }
}

async function pipe(bytes: Uint8Array, stream: CompressionStream | DecompressionStream): Promise<Uint8Array> {
  const body = new Response(bytes as Uint8Array<ArrayBuffer>).body;
  if (!body) throw new Error("no stream");
  return new Uint8Array(await new Response(body.pipeThrough(stream)).arrayBuffer());
}

function toBase64Url(bytes: Uint8Array): string {
  let bin = "";
  for (const b of bytes) bin += String.fromCharCode(b);
  return btoa(bin).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
}

function fromBase64Url(text: string): Uint8Array {
  const b64 = text.replace(/-/g, "+").replace(/_/g, "/");
  const bin = atob(b64 + "=".repeat((4 - (b64.length % 4)) % 4));
  return Uint8Array.from(bin, (c) => c.charCodeAt(0));
}

/** The link for a design's canonical JSON. Throws ShareTooLarge over the limit. */
export async function shareLink(json: string, base = SHARE_BASE): Promise<string> {
  const packed = await pipe(new TextEncoder().encode(json), new CompressionStream("deflate-raw"));
  const payload = toBase64Url(packed);
  if (payload.length > SHARE_LIMIT_BYTES) throw new ShareTooLarge(payload.length);
  return `${base}${SHARE_PREFIX}${payload}`;
}

/** The JSON inside a link's fragment (used by tests and, later, the web viewer). */
export async function readShareLink(link: string): Promise<string> {
  const at = link.indexOf(SHARE_PREFIX);
  if (at < 0) throw new Error("not a share link");
  const packed = fromBase64Url(link.slice(at + SHARE_PREFIX.length));
  return new TextDecoder().decode(await pipe(packed, new DecompressionStream("deflate-raw")));
}
