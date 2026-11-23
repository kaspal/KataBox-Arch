// Routes: code-based TanStack Router with hash history, so a reload under Tauri's asset
// protocol always lands on index.html. The Sandbox (React Flow) loads lazily.
import { validWhat } from "@/print/printing";
import type { PrintSearch } from "@/print/PrintPage";
import {
  createHashHistory,
  createRootRoute,
  createRoute,
  createRouter,
  lazyRouteComponent,
  type RouterHistory,
} from "@tanstack/react-router";
import { Shell } from "@/app/Shell";
import { Home } from "@/pages/Home";
import { Progress } from "@/pages/Placeholders";
import { ProblemsPage } from "@/pages/Problems";
import { Settings } from "@/pages/Settings";

const rootRoute = createRootRoute({ component: Shell });

const routeTree = rootRoute.addChildren([
  createRoute({ getParentRoute: () => rootRoute, path: "/", component: Home }),
  createRoute({ getParentRoute: () => rootRoute, path: "/problems", component: ProblemsPage }),
  createRoute({
    getParentRoute: () => rootRoute,
    path: "/problems/$id",
    component: lazyRouteComponent(() => import("@/solve/ProblemPage"), "ProblemPage"),
  }),
  createRoute({
    getParentRoute: () => rootRoute,
    path: "/sandbox",
    component: lazyRouteComponent(() => import("@/solve/SandboxPage"), "SandboxPage"),
  }),
  createRoute({
    getParentRoute: () => rootRoute,
    path: "/guide",
    component: lazyRouteComponent(() => import("@/guide/GuideIndex"), "GuideIndex"),
  }),
  createRoute({
    getParentRoute: () => rootRoute,
    path: "/guide/$kind",
    // `?section=sim` opens the entry at "In the simulator" (the Inspector's Learn more).
    validateSearch: (s: Record<string, unknown>): { section?: "sim" } => (s.section === "sim" ? { section: "sim" } : {}),
    component: lazyRouteComponent(() => import("@/guide/GuideEntryPage"), "GuideEntryPage"),
  }),
  createRoute({
    getParentRoute: () => rootRoute,
    path: "/decide/$id",
    component: lazyRouteComponent(() => import("@/guide/Decide"), "DecidePage"),
  }),
  createRoute({
    getParentRoute: () => rootRoute,
    path: "/glossary",
    // `?term=<id>`: a glossary link from content prose lands on that term.
    validateSearch: (s: Record<string, unknown>): { term?: string } =>
      typeof s.term === "string" && /^[a-z0-9-]{1,64}$/.test(s.term) ? { term: s.term } : {},
    component: lazyRouteComponent(() => import("@/guide/Decide"), "GlossaryPage"),
  }),
  createRoute({
    getParentRoute: () => rootRoute,
    path: "/numbers",
    component: lazyRouteComponent(() => import("@/practice/NumbersPage"), "NumbersPage"),
  }),
  createRoute({
    getParentRoute: () => rootRoute,
    path: "/review",
    component: lazyRouteComponent(() => import("@/practice/ReviewPage"), "ReviewPage"),
  }),
  createRoute({
    getParentRoute: () => rootRoute,
    path: "/paths",
    component: lazyRouteComponent(() => import("@/practice/PathsPage"), "PathsPage"),
  }),
  createRoute({
    getParentRoute: () => rootRoute,
    path: "/flaws",
    component: lazyRouteComponent(() => import("@/practice/FlawsPage"), "FlawsPage"),
  }),
  createRoute({
    getParentRoute: () => rootRoute,
    path: "/estimate",
    validateSearch: (s: Record<string, unknown>): { problem?: string } =>
      typeof s.problem === "string" && /^[a-z0-9-]{1,64}$/.test(s.problem) ? { problem: s.problem } : {},
    component: lazyRouteComponent(() => import("@/practice/EstimatePage"), "EstimatePage"),
  }),
  createRoute({
    getParentRoute: () => rootRoute,
    path: "/patterns",
    component: lazyRouteComponent(() => import("@/guide/Patterns"), "PatternsIndex"),
  }),
  createRoute({
    getParentRoute: () => rootRoute,
    path: "/patterns/$id",
    component: lazyRouteComponent(() => import("@/guide/Patterns"), "PatternPage"),
  }),
  createRoute({
    getParentRoute: () => rootRoute,
    path: "/samples",
    component: lazyRouteComponent(() => import("@/tour/SamplesPage"), "SamplesPage"),
  }),
  createRoute({
    getParentRoute: () => rootRoute,
    path: "/samples/$id",
    component: lazyRouteComponent(() => import("@/tour/SamplePage"), "SamplePage"),
  }),
  // The print layout (M7b.3): no app chrome; validated search only.
  createRoute({
    getParentRoute: () => rootRoute,
    path: "/print",
    validateSearch: (s: Record<string, unknown>): PrintSearch => ({
      what: validWhat(s.what) ? s.what : "guide",
      ...(typeof s.pdf === "number" && Number.isInteger(s.pdf) && s.pdf > 0 ? { pdf: s.pdf } : {}),
      ...(s.html === 1 ? { html: 1 as const } : {}),
      ...(typeof s.back === "string" && s.back.startsWith("/") && s.back.length < 512 ? { back: s.back } : {}),
      ...(typeof s.name === "string" ? { name: s.name.slice(0, 120) } : {}),
    }),
    component: lazyRouteComponent(() => import("@/print/PrintRoute"), "PrintRoute"),
  }),
  createRoute({ getParentRoute: () => rootRoute, path: "/progress", component: Progress }),
  createRoute({ getParentRoute: () => rootRoute, path: "/settings", component: Settings }),
  // Development builds only: the packet layer's frame-rate check (plan M1.4).
  ...(import.meta.env.DEV
    ? [
        createRoute({
          getParentRoute: () => rootRoute,
          path: "/dev/perf",
          component: lazyRouteComponent(() => import("@/pages/DevPerf"), "DevPerf"),
        }),
      ]
    : []),
]);

export function createAppRouter(history: RouterHistory = createHashHistory()) {
  return createRouter({ routeTree, history, defaultPreload: "intent" });
}

declare module "@tanstack/react-router" {
  interface Register {
    router: ReturnType<typeof createAppRouter>;
  }
}
