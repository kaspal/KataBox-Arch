// Component glyphs: one distinct line glyph per kind, on a 24 px grid at a 1.75 stroke
// (Lucide-compatible). Drawn for KataBox Arch (from the approved mockups, .handoff icons.json);
// flavours share their kind's glyph. Ids match the registry's `icon` field; a test checks
// every registry icon has a glyph here.
export const GLYPHS: Record<string, string> = {
  "client": "<rect x=\"2\" y=\"4\" width=\"14\" height=\"10\" rx=\"1.5\"/><path d=\"M6 18h6M9 14v4\"/><rect x=\"17\" y=\"8\" width=\"5\" height=\"11\" rx=\"1\"/>",
  "dns": "<path d=\"M12 3v18\"/><path d=\"M5 5h12l2.5 2.5L17 10H5z\"/><path d=\"M19 13H7l-2.5 2.5L7 18h12z\"/>",
  "cdn": "<circle cx=\"12\" cy=\"12\" r=\"9\"/><ellipse cx=\"12\" cy=\"12\" rx=\"4\" ry=\"9\"/><path d=\"M3 12h18\"/>",
  "waf": "<path d=\"M12 3l8 3v6c0 5-3.5 8-8 9-4.5-1-8-4-8-9V6z\"/><path d=\"M8 10h8M8 14h8M12 10v4\"/>",
  "lb": "<circle cx=\"12\" cy=\"4.5\" r=\"2\"/><path d=\"M12 6.5V11M5 11h14M5 11v4M12 11v4M19 11v4\"/><circle cx=\"5\" cy=\"17.5\" r=\"2\"/><circle cx=\"12\" cy=\"17.5\" r=\"2\"/><circle cx=\"19\" cy=\"17.5\" r=\"2\"/>",
  "gateway": "<path d=\"M4 21V9l8-6 8 6v12\"/><path d=\"M9 21v-7h6v7\"/>",
  "ratelimit": "<path d=\"M4 17a8 8 0 1 1 16 0\"/><path d=\"M12 17l4-5\"/><circle cx=\"12\" cy=\"17\" r=\"1.2\"/>",
  "app": "<rect x=\"3\" y=\"4\" width=\"18\" height=\"7\" rx=\"1.5\"/><rect x=\"3\" y=\"13\" width=\"18\" height=\"7\" rx=\"1.5\"/><path d=\"M7 7.5h.01M7 16.5h.01M11 7.5h6M11 16.5h6\"/>",
  "micro": "<rect x=\"8\" y=\"3\" width=\"8\" height=\"8\" rx=\"1.5\"/><rect x=\"3\" y=\"13\" width=\"8\" height=\"8\" rx=\"1.5\"/><rect x=\"13\" y=\"13\" width=\"8\" height=\"8\" rx=\"1.5\"/>",
  "worker": "<circle cx=\"12\" cy=\"12\" r=\"3\"/><path d=\"M12 2v3M12 19v3M2 12h3M19 12h3M4.9 4.9l2.1 2.1M17 17l2.1 2.1M4.9 19.1L7 17M17 7l2.1-2.1\"/>",
  "serverless": "<path d=\"M5 21l6.5-11.5\"/><path d=\"M8 3h3l9 18\"/>",
  "autoscale": "<rect x=\"7\" y=\"7\" width=\"14\" height=\"14\" rx=\"2\"/><path d=\"M4 17V5a1 1 0 0 1 1-1h12\"/><path d=\"M14 11v6M11 14h6\"/>",
  "sql": "<ellipse cx=\"12\" cy=\"5\" rx=\"8\" ry=\"3\"/><path d=\"M4 5v14c0 1.7 3.6 3 8 3s8-1.3 8-3V5\"/><path d=\"M4 12c0 1.7 3.6 3 8 3s8-1.3 8-3\"/>",
  "nosql": "<path d=\"M9 4c-2 0-2 1-2 3v2c0 1-1 2-2 3 1 1 2 2 2 3v2c0 2 0 3 2 3\"/><path d=\"M15 4c2 0 2 1 2 3v2c0 1 1 2 2 3-1 1-2 2-2 3v2c0 2 0 3-2 3\"/>",
  "cache": "<rect x=\"3\" y=\"6\" width=\"18\" height=\"11\" rx=\"1.5\"/><path d=\"M7 17v3M12 17v3M17 17v3\"/><path d=\"M13 8.5l-3 4h4l-3 3.5\"/>",
  "object": "<path d=\"M4 6l2 14h12l2-14\"/><ellipse cx=\"12\" cy=\"6\" rx=\"8\" ry=\"2.5\"/>",
  "search": "<circle cx=\"10.5\" cy=\"10.5\" r=\"6.5\"/><path d=\"M20 20l-4.8-4.8M7.5 10.5h6\"/>",
  "queue": "<rect x=\"2\" y=\"8\" width=\"5\" height=\"8\" rx=\"1\"/><rect x=\"9.5\" y=\"8\" width=\"5\" height=\"8\" rx=\"1\"/><path d=\"M17 12h5M19.5 9.5L22 12l-2.5 2.5\"/>",
  "stream": "<path d=\"M2 7c2.5-2.5 4.5 2.5 7 0s4.5 2.5 7 0 4.5 2.5 6 1\"/><path d=\"M2 12c2.5-2.5 4.5 2.5 7 0s4.5 2.5 7 0 4.5 2.5 6 1\"/><path d=\"M2 17c2.5-2.5 4.5 2.5 7 0s4.5 2.5 7 0 4.5 2.5 6 1\"/>",
  "pubsub": "<circle cx=\"12\" cy=\"12\" r=\"2\"/><path d=\"M16.2 7.8a6 6 0 0 1 0 8.4M7.8 16.2a6 6 0 0 1 0-8.4M19 5a10 10 0 0 1 0 14M5 19A10 10 0 0 1 5 5\"/>",
  "dlq": "<path d=\"M3 13h5l2 3h4l2-3h5\"/><path d=\"M5.5 5h13L21 13v6H3v-6z\"/><path d=\"M10 7.5l4 4M14 7.5l-4 4\"/>",
  "scheduler": "<circle cx=\"12\" cy=\"13\" r=\"8\"/><path d=\"M12 9v4l3 2M5 3L3 5M19 3l2 2\"/>",
  "metrics": "<path d=\"M3 12h4l3-8 4 16 3-8h4\"/>",
  "logs": "<rect x=\"5\" y=\"3\" width=\"14\" height=\"18\" rx=\"2\"/><path d=\"M9 8h6M9 12h6M9 16h4\"/>",
  "tracing": "<circle cx=\"6\" cy=\"19\" r=\"2\"/><circle cx=\"18\" cy=\"5\" r=\"2\"/><path d=\"M6 17v-5a3 3 0 0 1 3-3h6a3 3 0 0 0 3-3\"/>",
  "note": "<path d=\"M5 4h14v10l-6 6H5z\"/><path d=\"M13 20v-6h6\"/><path d=\"M8 9h8M8 12h4\"/>",
  "zone": "<rect x=\"3\" y=\"3\" width=\"18\" height=\"18\" rx=\"3\" stroke-dasharray=\"3 3\"/><path d=\"M7 8h5\"/>",
};
