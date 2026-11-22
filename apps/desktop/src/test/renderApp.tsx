import { render } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { ReactElement } from "react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { createMemoryHistory, createRootRoute, createRoute, createRouter, RouterProvider } from "@tanstack/react-router";
import App from "@/App";

/** Render the whole app at `path` (memory history). `@/lib/api` must be mocked. */
export function renderApp(path = "/") {
  const history = createMemoryHistory({ initialEntries: [path] });
  const user = userEvent.setup();
  const view = render(<App history={history} />);
  return { ...view, user, history };
}

/**
 * Render one component with the app's providers (React Query and a router with a /glossary
 * route), without the whole shell. `@/lib/api` must be mocked.
 */
export function renderWithProviders(ui: ReactElement) {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  const root = createRootRoute({ component: () => ui });
  const router = createRouter({
    routeTree: root.addChildren([createRoute({ getParentRoute: () => root, path: "/glossary" })]),
    history: createMemoryHistory({ initialEntries: ["/"] }),
  });
  return render(
    <QueryClientProvider client={client}>
      <RouterProvider router={router} />
    </QueryClientProvider>,
  );
}
