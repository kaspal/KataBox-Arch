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
import { Guide, Problems, Progress, Samples } from "@/pages/Placeholders";
import { Settings } from "@/pages/Settings";

const rootRoute = createRootRoute({ component: Shell });

const routeTree = rootRoute.addChildren([
  createRoute({ getParentRoute: () => rootRoute, path: "/", component: Home }),
  createRoute({ getParentRoute: () => rootRoute, path: "/problems", component: Problems }),
  createRoute({
    getParentRoute: () => rootRoute,
    path: "/sandbox",
    component: lazyRouteComponent(() => import("@/solve/SandboxPage"), "SandboxPage"),
  }),
  createRoute({ getParentRoute: () => rootRoute, path: "/guide", component: Guide }),
  createRoute({ getParentRoute: () => rootRoute, path: "/samples", component: Samples }),
  createRoute({ getParentRoute: () => rootRoute, path: "/progress", component: Progress }),
  createRoute({ getParentRoute: () => rootRoute, path: "/settings", component: Settings }),
]);

export function createAppRouter(history: RouterHistory = createHashHistory()) {
  return createRouter({ routeTree, history, defaultPreload: "intent" });
}

declare module "@tanstack/react-router" {
  interface Register {
    router: ReturnType<typeof createAppRouter>;
  }
}
