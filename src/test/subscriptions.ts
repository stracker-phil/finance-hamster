import type { EndedRowView, PlanChanged, SubscriptionDetail, SubscriptionForm, SubscriptionGroup, SubscriptionRowView, SubscriptionsView } from "../api";
import { ITEMS } from "./review";

/** A demo subscription form, with `patch` applied. */
export function subscriptionForm(patch: Partial<SubscriptionForm> = {}): SubscriptionForm {
  return {
    name: "Lumen AI",
    tone: 1,
    category: "AI & Software",
    active: true,
    endedOn: null,
    amount: "20.00",
    currency: "EUR",
    interval: "monthly",
    start: "2026-01-15",
    reminder: "off",
    itemKey: null,
    matchText: "LUMEN AI",
    exactOnly: false,
    ...patch,
  };
}

/** A demo list row, with `patch` applied. */
export function subscriptionRow(patch: Partial<SubscriptionRowView> = {}): SubscriptionRowView {
  return { id: "sub-1", name: "Lumen AI", letter: "L", tone: 1, anv: false, interval: "Monthly", change: null, next: "15 Dec", amount: "€20.00", ...patch };
}

/** A demo group of rows, with `patch` applied. */
export function subscriptionGroup(patch: Partial<SubscriptionGroup> = {}): SubscriptionGroup {
  return { label: "Next 30 days", total: "€20.00", rows: [subscriptionRow()], ...patch };
}

/** A demo ended row, with `patch` applied. */
export function endedRow(patch: Partial<EndedRowView> = {}): EndedRowView {
  return { id: "sub-9", name: "Old Streaming", letter: "O", tone: 2, interval: "Monthly", ended: "Ended 31 Mar", ...patch };
}

/** A demo subscriptions view, with `patch` applied. */
export function subscriptionsView(patch: Partial<SubscriptionsView> = {}): SubscriptionsView {
  return { summary: "1 active · €20.00 a month", groups: [subscriptionGroup()], ended: [], empty: false, categories: ["AI & Software", "Media"], items: ITEMS, ...patch };
}

/** A demo detail, with `patch` applied. */
export function subscriptionDetail(patch: Partial<SubscriptionDetail> = {}): SubscriptionDetail {
  return {
    id: "sub-1",
    name: "Lumen AI",
    letter: "L",
    tone: 1,
    interval: "Monthly",
    amount: "€20.00",
    next: "15 Dec",
    ended: null,
    year: 2026,
    timeline: [
      { label: "J", plan: 0, tip: "Jan · Plus" },
      { label: "F", plan: null, tip: "Feb" },
    ],
    plans: [{ name: "Plus", price: "€20.00 monthly", when: "since 15 Jan" }],
    anv: "Not for ANV",
    reminder: "Off",
    rule: "LUMEN AI",
    charges: [{ id: "tx-1", date: "15 Nov", account: "Private", amount: "−€20.00" }],
    form: subscriptionForm(),
    currentPlan: "Plus · €20.00 monthly",
    ...patch,
  };
}

/** A demo "plan changed?" hint on a review line, with `patch` applied. */
export function planChanged(patch: Partial<PlanChanged> = {}): PlanChanged {
  return { subscriptionId: "sub-1", name: "Lumen AI", text: "Lumen AI charged €25.00 instead of €20.00.", amount: "25.00", currency: "EUR", date: "2026-11-10", ...patch };
}
