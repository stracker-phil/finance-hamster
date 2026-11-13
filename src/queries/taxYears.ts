import { useQuery } from "@tanstack/react-query";
import { api } from "../api";

export const TAX_YEARS_KEY = ["tax-years"] as const;
export const RULES_KEY = ["rules-overview"] as const;

export const useTaxYears = () => useQuery({ queryKey: TAX_YEARS_KEY, queryFn: api.listTaxYears });

/** The rules files are edited outside the app, so the list is read afresh whenever it is shown. */
export const useRulesOverview = () => useQuery({ queryKey: RULES_KEY, queryFn: api.rulesOverview, staleTime: 0, refetchOnMount: "always", refetchOnWindowFocus: "always" });
