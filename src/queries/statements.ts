import { useQuery } from "@tanstack/react-query";
import { api } from "../api";

export const COVERAGE_KEY = ["statements-coverage"] as const;
export const ACCOUNT_LINES_KEY = ["account-lines"] as const;

export const useCoverage = () => useQuery({ queryKey: COVERAGE_KEY, queryFn: api.statementsCoverage });

export const useAccountLines = (accountId: string) => useQuery({ queryKey: [...ACCOUNT_LINES_KEY, accountId], queryFn: () => api.accountLines(accountId) });
