import { useEffect } from "react";
import { useQueryClient, type QueryKey } from "@tanstack/react-query";
import { api, type EventName } from "../api";
import { SETTINGS_KEY } from "./settings";

/** The query keys each Rust event makes stale. Features add their events and keys here. */
export const INVALIDATIONS: Partial<Record<EventName, QueryKey[]>> = {
  settings_changed: [SETTINGS_KEY],
};

/** Subscribes to every event of `INVALIDATIONS` while mounted and refetches the queries it names. */
export function useEvents() {
  const client = useQueryClient();
  useEffect(() => {
    const unlisten = Object.entries(INVALIDATIONS).map(([event, keys]) =>
      api.listen(event as EventName, () => {
        for (const queryKey of keys) void client.invalidateQueries({ queryKey }, { cancelRefetch: false });
      }),
    );
    return () => unlisten.forEach((p) => void p.then((fn) => fn()).catch(() => {}));
  }, [client]);
}
