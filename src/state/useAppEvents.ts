import { useEffect } from "react";
import { api, toAppError, type LaunchEvent } from "../api";
import { useStore } from "./store";

function apply(events: LaunchEvent[]) {
  for (const event of events) {
    if (event.type === "error") useStore.getState().setLaunchError(event.message);
  }
}

/** Takes the launch events queued before the page listened, then every later batch. */
export function useAppEvents() {
  useEffect(() => {
    let active = true;
    const take = () => {
      api.takeLaunchEvents().then(
        (events) => {
          if (active) apply(events);
        },
        (e: unknown) => {
          if (active) useStore.getState().setLaunchError(toAppError(e).message);
        },
      );
    };
    take();
    const unlisten = api.listen("launch", take);
    return () => {
      active = false;
      void unlisten.then((stop) => stop());
    };
  }, []);
}
