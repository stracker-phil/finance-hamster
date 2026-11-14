import { useQuery } from "@tanstack/react-query";
import { api } from "../api";

export const YEAR_CHECK_KEY = ["year-check"] as const;

export const useYearCheck = (year: number) => useQuery({ queryKey: [...YEAR_CHECK_KEY, year], queryFn: () => api.yearCheckView(year) });
