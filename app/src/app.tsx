import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { type AnyRouter, RouterProvider } from "@tanstack/react-router";
import { useState } from "react";
import { Toaster } from "sonner";
import { createAppRouter } from "./router";

export function createQueryClient() {
  return new QueryClient({
    defaultOptions: {
      queries: { retry: false, refetchOnWindowFocus: false, staleTime: 500 },
      mutations: { retry: false },
    },
  });
}

export function App({ router, queryClient }: { router?: AnyRouter; queryClient?: QueryClient }) {
  const [client] = useState(() => queryClient ?? createQueryClient());
  const [r] = useState(() => router ?? createAppRouter());
  return (
    <QueryClientProvider client={client}>
      <RouterProvider router={r} />
      <Toaster theme="system" position="bottom-right" richColors />
    </QueryClientProvider>
  );
}
