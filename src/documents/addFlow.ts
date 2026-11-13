import { useEffect } from "react";
import { api, toAppError } from "../api";
import { useStore } from "../state/store";

const showError = (e: unknown) => useStore.getState().showToast("error", toAppError(e).message);

/** Asks for an invoice PDF; a new one opens the Add document sheet. */
export function startAddDocument() {
  api
    .documentAddChoose()
    .then((preview) => {
      if (preview) useStore.getState().setAddPreview(preview);
    })
    .catch(showError);
}

/** Invoices dropped on the window arrive read by Rust: the sheet opens, or a toast says why not. */
export function useDroppedDocuments() {
  useEffect(() => {
    const unlisten = api.listen("add_ready", ({ preview, error }) => {
      if (preview) useStore.getState().setAddPreview(preview);
      else if (error) useStore.getState().showToast("error", error);
    });
    return () => void unlisten.then((stop) => stop()).catch(() => {});
  }, []);
}
