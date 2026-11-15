import { useQuery } from "@tanstack/react-query";
import { api } from "../api";

export const TAX_YEARS_KEY = ["tax-years"] as const;
export const RULES_KEY = ["rules-overview"] as const;

export const useTaxYears = () => useQuery({ queryKey: TAX_YEARS_KEY, queryFn: api.listTaxYears });

export const useRulesOverview = () => useQuery({ queryKey: RULES_KEY, queryFn: api.rulesOverview });
export const useRulesYear = (year: number) => useQuery({ queryKey: [...RULES_KEY, year], queryFn: () => api.rulesYear(year) });
