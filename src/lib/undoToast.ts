import { api, toAppError } from "../api";
import { useStore } from "../state/store";

export const showError = (e: unknown) => useStore.getState().showToast("error", toAppError(e).message);

const undo = () => {
  api
    .undo()
    .then((label) => label && useStore.getState().showToast("success", `Undid ${label}`))
    .catch(showError);
};

/** Says what changed, with Undo. */
export const undoToast = (label: string) => useStore.getState().showToast("success", label, { label: "Undo", run: undo });
