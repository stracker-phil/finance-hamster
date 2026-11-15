import { useQuery } from "@tanstack/react-query";
import { api } from "../api";

export const OTHERS_KEY = ["others"] as const;
export const OTHER_KEY = ["other"] as const;

export const useOthers = (query: string) => useQuery({ queryKey: [...OTHERS_KEY, query], queryFn: () => api.othersList(query), placeholderData: (previous) => previous });

export const useOther = (id: string, enabled = true) => useQuery({ queryKey: [...OTHER_KEY, id], queryFn: () => api.otherView(id), enabled, meta: { quiet: true } });
