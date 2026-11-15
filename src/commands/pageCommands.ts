import { useEffect } from "react";
import { api, toAppError } from "../api";
import { isTypingTarget } from "./pageKeys";
import { FILTER_FIELD_ID } from "../shell/Header";
import { startAddDocument } from "../documents/addFlow";
import { startImport } from "../statements/importFlow";
import { newSubscription } from "../subscriptions/flow";
import { useStore } from "../state/store";

type Handler = () => void;

const UNDO_WORDS = {
  undo: { done: "Undid", none: "Nothing to undo" },
  redo: { done: "Redid", none: "Nothing to redo" },
};

/** ⌘Z in a text field undoes the typing; elsewhere the newest change, as a new journaled op. */
function undoOrRedo(which: keyof typeof UNDO_WORDS) {
  if (isTypingTarget(document.activeElement)) {
    document.execCommand(which);
    return;
  }
  const { showToast } = useStore.getState();
  api[which]()
    .then((label) => {
      if (label) showToast("success", `${UNDO_WORDS[which].done} ${label}`);
      else showToast("info", UNDO_WORDS[which].none);
    })
    .catch((e: unknown) => showToast("error", toAppError(e).message));
}

/** What the main window does for each page command (`commands::COMMANDS` in Rust). */
const HANDLERS: Record<string, Handler> = {
  "edit:undo": () => undoOrRedo("undo"),
  "edit:redo": () => undoOrRedo("redo"),
  "go:today": () => useStore.getState().navigate({ page: "today" }),
  "go:subscriptions": () => useStore.getState().navigate({ page: "subscriptions" }),
  "go:statements": () => useStore.getState().navigate({ page: "statements" }),
  "go:documents": () => useStore.getState().navigate({ page: "documents" }),
  "go:review": () => useStore.getState().navigate({ page: "review" }),
  "go:back": () => useStore.getState().goBack(),
  "go:forward": () => useStore.getState().goForward(),
  "palette:open": () => useStore.getState().setPaletteOpen(true),
  "statement:import": startImport,
  "document:add": startAddDocument,
  "subscription:new": newSubscription,
  "filter:focus": () => document.getElementById(FILTER_FIELD_ID)?.focus(),
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
