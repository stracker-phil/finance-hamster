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

/** Asks for the invoice of one bank line; the sheet starts on that line instead of its own best guess. */
export async function attachInvoiceTo(lineId: string) {
  try {
    const preview = await api.documentAddChoose();
    if (!preview) return;
    const line = await api.documentLineChoice(lineId);
    const lines = line ? [line, ...preview.lines.filter((l) => l.id !== line.id)] : preview.lines;
    useStore.getState().setAddPreview({ ...preview, lines });
  } catch (e) {
    showError(e);
  }
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
