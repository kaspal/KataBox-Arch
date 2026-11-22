// Routes: code-based TanStack Router with hash history, so a reload under Tauri's asset
// protocol always lands on index.html. The Sandbox (React Flow) loads lazily.
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
    path: "/samples",
    component: lazyRouteComponent(() => import("@/tour/SamplesPage"), "SamplesPage"),
  }),
  createRoute({
    getParentRoute: () => rootRoute,
    path: "/samples/$id",
    component: lazyRouteComponent(() => import("@/tour/SamplePage"), "SamplePage"),
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
