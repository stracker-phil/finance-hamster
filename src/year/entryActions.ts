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

export function deleteEntry(year: number, id: string) {
  api.yearEntryDelete(year, id).then(undoToast).catch(showError);
}

/** Asks for a PDF and attaches it to the amount. */
export function attachEvidence(year: number, id: string) {
  api
    .yearEvidenceChoose()
    .then((path) => (path ? api.yearEntryAttach(year, id, path).then(undoToast) : undefined))
    .catch(showError);
}

export function openEvidence(year: number, id: string) {
  api.yearEvidenceOpen(year, id).catch(showError);
}
