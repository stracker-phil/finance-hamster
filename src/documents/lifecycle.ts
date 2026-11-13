import { api, toAppError, type DocumentStep } from "../api";
import { useStore } from "../state/store";

/** Runs one lifecycle step and says how it went. */
export function runStep(id: string, step: DocumentStep) {
  const { showToast } = useStore.getState();
  return api
    .documentStep(id, step)
    .then((label) => showToast("success", label))
    .catch((e: unknown) => showToast("error", toAppError(e).message));
}

/** Unlocks a finished document after asking. */
export async function unlock(id: string) {
  const confirmed = await useStore.getState().confirm({ title: "Unlock this document?", text: "Its details can be changed again; the change is written to its History.", action: "Unlock" });
  if (confirmed) await runStep(id, { kind: "unlock" });
}
