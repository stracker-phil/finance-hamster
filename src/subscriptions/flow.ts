import { api, type SubscriptionDetail } from "../api";
import { showError, undoToast } from "../lib/undoToast";
import { useStore } from "../state/store";
import { today } from "../lib/today";

/** Opens the New sheet, empty. */
export function newSubscription() {
  useStore.getState().setSubscriptionSheet({ kind: "new", form: null });
}

/** Opens the New sheet prefilled from a bank line. */
/** Opens New Subscription from a bank line; `term` is the text it should match, else the line's key. */
export function newSubscriptionFrom(lineId: string, term: string | null = null) {
  api
    .subscriptionPrefill(lineId, term)
    .then((form) => useStore.getState().setSubscriptionSheet({ kind: "new", form }))
    .catch(showError);
}

export function editSubscription(detail: SubscriptionDetail) {
  useStore.getState().setSubscriptionSheet({ kind: "edit", id: detail.id, form: detail.form });
}

/** Opens Change Plan, starting today with the current plan's values unless `from` gives the new amount. */
export function changePlan(detail: SubscriptionDetail, from?: { amount: string; currency: string; date: string }) {
  useStore.getState().setSubscriptionSheet({
    kind: "plan",
    id: detail.id,
    name: detail.name,
    currentPlan: detail.currentPlan,
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
