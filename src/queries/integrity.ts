import { useQuery } from "@tanstack/react-query";
import { api } from "../api";

export const FILE_CHECK_KEY = ["file-check"] as const;
export const STATUS_KEY = ["status"] as const;

export const useLastFileCheck = () => useQuery({ queryKey: FILE_CHECK_KEY, queryFn: api.filesLastCheck });

export const useStatus = () => useQuery({ queryKey: STATUS_KEY, queryFn: api.statusView });
