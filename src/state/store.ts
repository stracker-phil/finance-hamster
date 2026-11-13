import { create } from "zustand";
import type { AppInfo } from "../api";

/** UI-only state of the main window; data reads go through TanStack Query (ADR 0014). */
export interface AppStore {
  appInfo: AppInfo | null;
  /** The last error a launch reported (`hamster show` failing after it started the app). */
  launchError: string | null;
  setAppInfo: (info: AppInfo) => void;
  setLaunchError: (message: string | null) => void;
}

export const useStore = create<AppStore>((set) => ({
  appInfo: null,
  launchError: null,
  setAppInfo: (appInfo) => set({ appInfo }),
  setLaunchError: (launchError) => set({ launchError }),
}));
