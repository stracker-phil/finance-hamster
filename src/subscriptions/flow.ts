import { api, type ReviewLine, type SubscriptionDetail } from "../api";
import { showError, undoToast } from "../lib/undoToast";
import { useStore } from "../state/store";
import { today } from "../lib/today";

/** Opens the New sheet, empty. */
export function newSubscription() {
  useStore.getState().setSubscriptionSheet({ kind: "new", form: null });
}

/** Opens New Subscription from a bank line, which the sheet shows; `term` is the text it should match, else the
 *  line's key. */
export function newSubscriptionFrom(line: ReviewLine, term: string | null = null) {
  api
    .subscriptionPrefill(line.id, term)
    .then((form) => useStore.getState().setSubscriptionSheet({ kind: "new", form, source: line }))
    .catch(showError);
}

export function editSubscription(detail: SubscriptionDetail) {
  useStore.getState().setSubscriptionSheet({ kind: "edit", id: detail.id, form: detail.form });
}

/** Opens Change Plan, starting today with the current plan's values unless `from` gives the new amount; `source`
 *  is the bank line it was opened from. */
export function changePlan(detail: SubscriptionDetail, from?: { amount: string; currency: string; date: string }, source?: ReviewLine) {
  useStore.getState().setSubscriptionSheet({
    kind: "plan",
    id: detail.id,
    name: detail.name,
    currentPlan: detail.currentPlan,
    source,
    form: {
      name: "",
      amount: from?.amount ?? detail.form.amount,
      currency: from?.currency ?? detail.form.currency,
      interval: detail.form.interval,
      start: from?.date ?? today(),
    },
  });
}

/** Ends the subscription today after asking; nothing is deleted. */
export async function endSubscription(id: string, name: string) {
  const confirmed = await useStore.getState().confirm({
    title: `End ${name}?`,
    text: "It ends today. Its plans and charges stay as history, and Resume brings it back.",
    action: "End Subscription",
  });
  if (confirmed) api.subscriptionSetEnded(id, today()).then(undoToast).catch(showError);
}

export function resumeSubscription(id: string) {
  api.subscriptionSetEnded(id, null).then(undoToast).catch(showError);
}

/** Deletes the subscription after asking; its charges go back to Review. */
export async function deleteSubscription(id: string, name: string) {
  const confirmed = await useStore.getState().confirm({
    title: `Delete ${name}?`,
    text: "Its plans go, and its charges wait in Review again. To keep the history, end it instead.",
    action: "Delete",
  });
  if (!confirmed) return false;
  try {
    undoToast(await api.subscriptionDelete(id));
    return true;
  } catch (e) {
    showError(e);
    return false;
  }
}
