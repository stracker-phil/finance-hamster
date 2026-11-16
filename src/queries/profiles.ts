import { useQuery } from "@tanstack/react-query";
import { api } from "../api";

export const PROFILES_KEY = ["profiles"] as const;

export const useProfiles = () => useQuery({ queryKey: PROFILES_KEY, queryFn: api.profilesList });
