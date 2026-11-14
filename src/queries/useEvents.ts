import { useEffect } from "react";
import { useQueryClient, type QueryClient, type QueryKey } from "@tanstack/react-query";
import { api, type EventName } from "../api";
import { ACCOUNTS_KEY } from "./accounts";
import { COMMANDS_KEY } from "./commands";
import { ARCHIVE_KEY, DELETIONS_KEY, DOCUMENT_KEY, DOCUMENT_LINES_KEY, DOCUMENTS_KEY, HISTORY_KEY, LOG_KEY, VENDORS_KEY } from "./documents";
import { FILE_CHECK_KEY, STATUS_KEY } from "./integrity";
import { LINE_RULES_KEY } from "./lineRules";
import { REVIEW_KEY } from "./review";
import { TODAY_KEY } from "./today";
import { YEAR_CHECK_KEY } from "./yearCheck";
import { ELSEWHERE_KEY, YEAR_KEY } from "./year";
import { SETTINGS_KEY } from "./settings";
import { MATCH_COUNT_KEY, SUBSCRIPTION_KEY, SUBSCRIPTIONS_KEY } from "./subscriptions";
import { ACCOUNT_LINES_KEY, COVERAGE_KEY } from "./statements";
import { RULES_KEY, TAX_YEARS_KEY } from "./taxYears";

/** The query keys each Rust event makes stale; a feature adds its event here, or its table to `TABLE_KEYS`. */
export const INVALIDATIONS: Partial<Record<EventName, QueryKey[]>> = {
  settings_changed: [SETTINGS_KEY, TODAY_KEY, STATUS_KEY],
  files_checked: [FILE_CHECK_KEY, STATUS_KEY],
  commands_changed: [COMMANDS_KEY],
};

/** The query keys a `data_changed` for each table makes stale. */
export const TABLE_KEYS: Record<string, QueryKey[]> = {
  accounts: [ACCOUNTS_KEY, COVERAGE_KEY, ACCOUNT_LINES_KEY, LINE_RULES_KEY, REVIEW_KEY, TODAY_KEY, STATUS_KEY],
  statements: [COVERAGE_KEY, ACCOUNT_LINES_KEY, TODAY_KEY, STATUS_KEY],
  transactions: [MATCH_COUNT_KEY, SUBSCRIPTIONS_KEY, SUBSCRIPTION_KEY, ACCOUNT_LINES_KEY, LINE_RULES_KEY, COVERAGE_KEY, REVIEW_KEY, DOCUMENT_LINES_KEY, YEAR_KEY, ELSEWHERE_KEY, TODAY_KEY, STATUS_KEY],
  rules: [LINE_RULES_KEY, TODAY_KEY, STATUS_KEY],
  decisions: [LINE_RULES_KEY, COVERAGE_KEY, ACCOUNT_LINES_KEY, REVIEW_KEY, YEAR_KEY, ELSEWHERE_KEY, SUBSCRIPTIONS_KEY, SUBSCRIPTION_KEY, MATCH_COUNT_KEY, TODAY_KEY, STATUS_KEY],
  documents: [DOCUMENT_KEY, DOCUMENT_LINES_KEY, DOCUMENTS_KEY, VENDORS_KEY, HISTORY_KEY, LOG_KEY, ARCHIVE_KEY, YEAR_KEY, ELSEWHERE_KEY, TODAY_KEY, STATUS_KEY],
  vendors: [DOCUMENT_KEY, DOCUMENTS_KEY, VENDORS_KEY, HISTORY_KEY, LOG_KEY, YEAR_KEY, ELSEWHERE_KEY, TODAY_KEY, STATUS_KEY],
  document_links: [DOCUMENT_KEY, DOCUMENT_LINES_KEY, DOCUMENTS_KEY, HISTORY_KEY, LOG_KEY, YEAR_KEY, ELSEWHERE_KEY, TODAY_KEY, STATUS_KEY],
  deletions: [DELETIONS_KEY, LOG_KEY, ARCHIVE_KEY, YEAR_KEY, ELSEWHERE_KEY, TODAY_KEY, STATUS_KEY],
  answers: [YEAR_KEY, YEAR_CHECK_KEY, TODAY_KEY, STATUS_KEY],
  year_checks: [YEAR_CHECK_KEY, TODAY_KEY],
  entries: [YEAR_KEY, TODAY_KEY, STATUS_KEY],
  subscriptions: [SUBSCRIPTIONS_KEY, SUBSCRIPTION_KEY, MATCH_COUNT_KEY, REVIEW_KEY, ACCOUNT_LINES_KEY, YEAR_KEY, TODAY_KEY, STATUS_KEY],
  plans: [SUBSCRIPTIONS_KEY, SUBSCRIPTION_KEY, MATCH_COUNT_KEY, REVIEW_KEY, ACCOUNT_LINES_KEY, TODAY_KEY, STATUS_KEY],
  tax_years: [YEAR_CHECK_KEY, TAX_YEARS_KEY, RULES_KEY, LINE_RULES_KEY, REVIEW_KEY, ACCOUNT_LINES_KEY, DOCUMENT_KEY, DOCUMENTS_KEY, VENDORS_KEY, YEAR_KEY, ELSEWHERE_KEY, TODAY_KEY, STATUS_KEY],
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
