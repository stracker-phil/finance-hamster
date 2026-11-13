import { useEffect } from "react";
import { api } from "../api";
import { useStore } from "./store";

/** Loads the app's name and version once. */
export function useAppInfo() {
  useEffect(() => {
    let active = true;
    api.appInfo().then(
      (info) => {
        if (active) useStore.getState().setAppInfo(info);
      },
      () => {},
    );
    return () => {
      active = false;
    };
  }, []);
}
