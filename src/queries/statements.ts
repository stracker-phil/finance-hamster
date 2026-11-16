import { useQuery } from "@tanstack/react-query";
import { api } from "../api";

export const COVERAGE_KEY = ["statements-coverage"] as const;
export const ACCOUNT_LINES_KEY = ["account-lines"] as const;

/** The accounts' months in `year`, or in this year without one; `query` keeps the accounts whose name contains
 *  it, and the sidebar's badge asks for all. */
export const useCoverage = (query = "", year?: number) =>
  useQuery({ queryKey: [...COVERAGE_KEY, query, year ?? null], queryFn: () => api.statementsCoverage(query, year), placeholderData: (previous) => previous });

export const useAccountLines = (accountId: string, query = "") =>
  useQuery({ queryKey: [...ACCOUNT_LINES_KEY, accountId, query], queryFn: () => api.accountLines(accountId, query), placeholderData: (previous) => previous });

export const STATEMENTS_LIST_KEY = ["statements-list"] as const;
export const STATEMENT_KEY = ["statement"] as const;

export const useStatementsList = (query: string) => useQuery({ queryKey: [...STATEMENTS_LIST_KEY, query], queryFn: () => api.statementsList(query), placeholderData: (previous) => previous });

export const useStatement = (id: string, enabled = true) => useQuery({ queryKey: [...STATEMENT_KEY, id], queryFn: () => api.statementView(id), enabled, meta: { quiet: true } });
