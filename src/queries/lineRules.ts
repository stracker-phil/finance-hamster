import { useQuery } from "@tanstack/react-query";
import { api } from "../api";

/** The rules that sort statement lines (Settings → Rules); the tax rules files are `RULES_KEY`. */
export const LINE_RULES_KEY = ["line-rules"] as const;

export const useLineRules = () => useQuery({ queryKey: LINE_RULES_KEY, queryFn: api.rulesView });
