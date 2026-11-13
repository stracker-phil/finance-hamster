import { useQuery } from "@tanstack/react-query";
import { api } from "../api";

export const ACCOUNTS_KEY = ["accounts"] as const;

export const useAccounts = () => useQuery({ queryKey: ACCOUNTS_KEY, queryFn: api.listAccounts });
