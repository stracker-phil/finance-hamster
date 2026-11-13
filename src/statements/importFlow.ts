import { useEffect } from "react";
import { api, toAppError } from "../api";
import { useStore } from "../state/store";

const showError = (e: unknown) => useStore.getState().showToast("error", toAppError(e).message);

/** Asks for a statement PDF; a readable one opens the Import sheet. */
export function startImport() {
  api
    .statementImportChoose()
    .then((preview) => {
      if (preview) useStore.getState().setImportPreview(preview);
    })
    .catch(showError);
}

/** Statements dropped on the window arrive read by Rust: the sheet opens, or a toast says why not. */
export function useDroppedStatements() {
  useEffect(() => {
    const unlisten = api.listen("import_ready", ({ preview, error }) => {
      if (preview) useStore.getState().setImportPreview(preview);
      else if (error) useStore.getState().showToast("error", error);
    });
    return () => void unlisten.then((stop) => stop()).catch(() => {});
  }, []);
}
