import { useQuery } from "@tanstack/react-query";
import { api } from "../api";

export const YEAR_KEY = ["year-page"] as const;
export const ELSEWHERE_KEY = ["elsewhere"] as const;

export const useYearPage = (year: number, query: string) =>
  useQuery({ queryKey: [...YEAR_KEY, year, query], queryFn: () => api.yearPage(year, query), placeholderData: (previous) => previous, meta: { quiet: true } });

export const useElsewhere = (query: string) => useQuery({ queryKey: [...ELSEWHERE_KEY, query], queryFn: () => api.searchElsewhere(query), enabled: query.trim() !== "" });
