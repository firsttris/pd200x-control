import {
  createHashHistory,
  createRootRoute,
  createRoute,
  createRouter,
  type RouterHistory,
} from "@tanstack/react-router";
import { AppShell } from "./components/app-shell";
import { BasicsPage } from "./routes/basics";
import { LightPage } from "./routes/light";
import { SoundPage } from "./routes/sound";

const rootRoute = createRootRoute({ component: AppShell });
const basicsRoute = createRoute({ getParentRoute: () => rootRoute, path: "/", component: BasicsPage });
const soundRoute = createRoute({ getParentRoute: () => rootRoute, path: "/sound", component: SoundPage });
const lightRoute = createRoute({ getParentRoute: () => rootRoute, path: "/light", component: LightPage });

const routeTree = rootRoute.addChildren([basicsRoute, soundRoute, lightRoute]);

export function createAppRouter(history: RouterHistory = createHashHistory()) {
  return createRouter({ routeTree, history, defaultPreload: false });
}

declare module "@tanstack/react-router" {
  interface Register {
    router: ReturnType<typeof createAppRouter>;
  }
}
