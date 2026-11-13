import { api } from "../api";
import { showError, undoToast } from "../lib/undoToast";

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
