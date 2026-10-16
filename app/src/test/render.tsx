import { createMemoryHistory } from "@tanstack/react-router";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { App, createQueryClient } from "@/app";
import type { MockBackend } from "@/lib/mock/backend";
import { installMockBackend } from "@/lib/mock/install";
import { createAppRouter } from "@/router";

/** Renders the whole app at `path` against a fresh mock backend. */
export async function renderApp(path = "/", setup?: (backend: MockBackend) => void) {
  const backend = installMockBackend({ latency: 0 });
  setup?.(backend);
  const router = createAppRouter(createMemoryHistory({ initialEntries: [path] }));
  const user = userEvent.setup();
  const utils = render(<App router={router} queryClient={createQueryClient()} />);
  await screen.findByRole("navigation", { name: "Bereiche" });
  return { ...utils, backend, router, user };
}

/** The last `apply_change` the frontend sent. */
export function lastChange(backend: MockBackend) {
  const calls = backend.state.calls.filter((c) => c.cmd === "apply_change");
  return calls.at(-1)?.args.change;
}
