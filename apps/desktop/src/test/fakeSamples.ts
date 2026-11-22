// A two-step sample for UI tests (the shapes Rust sends: ka-app samples.rs).
import type { Design, Sample, SampleSummary } from "@/bindings";

const card = (id: string, kind: string, flavour: string, x: number, y: number) =>
  ({ id, kind, flavour, label: "", position: { x, y }, props: {}, group_id: null }) as Design["nodes"][number];

export const ONE: Design = {
  schema_version: 2,
  id: "hello-one",
  name: "Hello (one server)",
  problem_id: null,
  nodes: [card("client", "client", "web", 0, 0), card("app", "app", "go", 0, 200)],
  edges: [{ id: "e1", from: "client", to: "app", protocol: "http" }],
  groups: [],
  notes: [],
  viewport: { x: 0, y: 0, zoom: 1 },
};

export const TWO: Design = {
  ...ONE,
  id: "hello-two",
  name: "Hello (two servers)",
  nodes: [card("client", "client", "web", 0, 0), card("lb", "lb", "nginx", 0, 160), card("app", "app", "go", -150, 320), card("app2", "app", "go", 150, 320)],
  edges: [
    { id: "e1", from: "client", to: "lb", protocol: "http" },
    { id: "e2", from: "lb", to: "app", protocol: "http" },
    { id: "e3", from: "lb", to: "app2", protocol: "http" },
  ],
};

export const FAKE_SAMPLE: Sample = {
  def: {
    id: "hello-traffic",
    number: 1,
    title: "Hello, Traffic",
    title_es: "Hola, tráfico",
    summary: "One server meets a crowd.",
    summary_es: "Un servidor se encuentra con una multitud.",
    problem: "url-shortener",
    tour: true,
    steps: [
      {
        snapshot: "one.kbarch",
        title: "One server",
        title_es: "Un servidor",
        caption: "Requests leave the client and reach the server.",
        caption_es: "Las peticiones salen del cliente y llegan al servidor.",
        focus: ["app"],
        run: { preset: "spike", start_s: 18, speed: 1 },
        callout: { metric: "p95_ms", after: 420, target: 80 },
      },
      {
        snapshot: "two.kbarch",
        title: "Share the load",
        title_es: "Reparte la carga",
        caption: "A load balancer spreads requests over two servers.",
        caption_es: "Un balanceador reparte las peticiones entre dos servidores.",
        focus: [],
        camera: { x: 100, y: 200, zoom: 1 },
        run: { test: "normal", start_s: 0, speed: 2 },
        callout: { metric: "p95_ms", before: 420, after: 18, target: 80 },
      },
    ],
  },
  snapshots: { "one.kbarch": ONE, "two.kbarch": TWO },
};

export const FAKE_SAMPLE_SUMMARY: SampleSummary = {
  id: "hello-traffic",
  number: 1,
  title: "Hello, Traffic",
  title_es: "Hola, tráfico",
  summary: "One server meets a crowd.",
  summary_es: "Un servidor se encuentra con una multitud.",
  tour: true,
  steps: 2,
  problem: "url-shortener",
  preview: ONE,
  preview_run: { preset: "spike", start_s: 18, speed: 1 },
};
