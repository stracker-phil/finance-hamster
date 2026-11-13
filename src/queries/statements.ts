import { useQuery } from "@tanstack/react-query";
import { api } from "../api";

export const COVERAGE_KEY = ["statements-coverage"] as const;
export const ACCOUNT_LINES_KEY = ["account-lines"] as const;

/** `query` keeps the accounts whose name contains it; the sidebar's badge asks for all. */
export const useCoverage = (query = "") => useQuery({ queryKey: [...COVERAGE_KEY, query], queryFn: () => api.statementsCoverage(query), placeholderData: (previous) => previous });

export const useAccountLines = (accountId: string, query = "") =>
  useQuery({ queryKey: [...ACCOUNT_LINES_KEY, accountId, query], queryFn: () => api.accountLines(accountId, query), placeholderData: (previous) => previous });
