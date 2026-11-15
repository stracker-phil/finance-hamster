import { api, toAppError, type DocumentStep } from "../api";
import { useStore } from "../state/store";
import { t } from "../i18n";

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
  const text = t().documents.unlock;
  const confirmed = await useStore.getState().confirm({ title: text.title, text: text.text, action: text.action });
  if (confirmed) await runStep(id, { kind: "unlock" });
}
