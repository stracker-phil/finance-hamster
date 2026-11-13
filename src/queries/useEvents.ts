import { useEffect } from "react";
import { useQueryClient, type QueryClient, type QueryKey } from "@tanstack/react-query";
import { api, type EventName } from "../api";
import { ACCOUNTS_KEY } from "./accounts";
import { SETTINGS_KEY } from "./settings";
import { ACCOUNT_LINES_KEY, COVERAGE_KEY } from "./statements";
import { RULES_KEY, TAX_YEARS_KEY } from "./taxYears";

/** The query keys each Rust event makes stale; a feature adds its event here, or its table to `TABLE_KEYS`. */
export const INVALIDATIONS: Partial<Record<EventName, QueryKey[]>> = {
  settings_changed: [SETTINGS_KEY],
};

/** The query keys a `data_changed` for each table makes stale. */
export const TABLE_KEYS: Record<string, QueryKey[]> = {
  accounts: [ACCOUNTS_KEY, COVERAGE_KEY, ACCOUNT_LINES_KEY],
  statements: [COVERAGE_KEY, ACCOUNT_LINES_KEY],
  transactions: [ACCOUNT_LINES_KEY],
  tax_years: [TAX_YEARS_KEY, RULES_KEY],
};

function invalidate(client: QueryClient, keys: QueryKey[]) {
  for (const queryKey of keys) void client.invalidateQueries({ queryKey }, { cancelRefetch: false });
}

/** Subscribes to the invalidation events and `data_changed` while mounted; refetches the queries they name. */
export function useEvents() {
  const client = useQueryClient();
  useEffect(() => {
    const unlisten = [
      ...Object.entries(INVALIDATIONS).map(([event, keys]) => api.listen(event as EventName, () => invalidate(client, keys))),
      api.listen("data_changed", ({ tables }) => invalidate(client, tables.flatMap((t) => TABLE_KEYS[t] ?? []))),
    ];
    return () => unlisten.forEach((p) => void p.then((fn) => fn()).catch(() => {}));
  }, [client]);
}
