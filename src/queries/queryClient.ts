import { QueryCache, QueryClient } from "@tanstack/react-query";
import { toAppError } from "../api";
import { useStore } from "../state/store";

/** Data changes only when Rust says so (an event, see `useEvents`): no retries, never stale, no refetch on focus.
 *  A failed read shows as an error toast, unless its query says the page shows the failure itself
 *  (`meta: { quiet: true }`). */
export function createQueryClient(): QueryClient {
  return new QueryClient({
    queryCache: new QueryCache({
      onError: (error, query) => {
        if (!query.meta?.quiet) useStore.getState().showToast("error", toAppError(error).message);
      },
    }),
    defaultOptions: {
      queries: { retry: false, staleTime: Infinity, refetchOnWindowFocus: false },
      mutations: { retry: false },
    },
  });
}

/** The page's one client; tests make a fresh one with `createQueryClient`. */
export const queryClient = createQueryClient();
