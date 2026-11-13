import { useQuery } from "@tanstack/react-query";
import { api } from "../api";

export const TODAY_KEY = ["today"] as const;

export const useToday = () => useQuery({ queryKey: TODAY_KEY, queryFn: api.todayView });
