import { useMutation, useQuery } from "@tanstack/react-query";
import { api, toAppError, type SettingsPatch } from "../api";
import { useStore } from "../state/store";

export const SETTINGS_KEY = ["settings"] as const;

export const useDataFolder = () => useQuery({ queryKey: ["data-folder"], queryFn: api.dataFolder });

export const useSettings = () => useQuery({ queryKey: SETTINGS_KEY, queryFn: api.getSettings });

/** Saves a change; the `settings_changed` event refreshes every window. */
export function useUpdateSettings() {
  return useMutation({
    mutationFn: (patch: SettingsPatch) => api.updateSettings(patch),
    onError: (error) => useStore.getState().showToast("error", toAppError(error).message),
  });
}
