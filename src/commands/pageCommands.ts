import { useEffect } from "react";
import { api } from "../api";
import { FILTER_FIELD_ID } from "../shell/Header";
import { useStore } from "../state/store";

type Handler = () => void;

/** What the main window does for each page command (`commands::COMMANDS` in Rust). */
const HANDLERS: Record<string, Handler> = {
  "go.today": () => useStore.getState().navigate({ page: "today" }),
  "go.subscriptions": () => useStore.getState().navigate({ page: "subscriptions" }),
  "go.statements": () => useStore.getState().navigate({ page: "statements" }),
  "go.documents": () => useStore.getState().navigate({ page: "documents" }),
  "go.back": () => useStore.getState().goBack(),
  "go.forward": () => useStore.getState().goForward(),
  "palette.open": () => useStore.getState().setPaletteOpen(true),
  "filter.focus": () => document.getElementById(FILTER_FIELD_ID)?.focus(),
};

export function runPageCommand(id: string) {
  HANDLERS[id]?.();
}

/** Carries out the page commands Rust sends (menus, shortcuts, the palette, the CLI). */
export function usePageCommands() {
  useEffect(() => {
    const unlisten = api.listen("run_command", runPageCommand);
    return () => void unlisten.then((stop) => stop()).catch(() => {});
  }, []);
}
