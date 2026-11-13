import type { ItemOption, ReviewLine, ReviewQueue } from "../api";

/** A demo line waiting for a decision, with `patch` applied. */
export function reviewLine(patch: Partial<ReviewLine> = {}): ReviewLine {
  return {
    id: "line-1",
    accountName: "Private",
    accountIndex: 0,
    date: "Tue, 10 Nov",
    title: "Example Bookshop",
    amount: "−€34.90",
    raw: "Example Bookshop Vienna\nBooking text 4711",
    key: "EXAMPLEBOOKS",
    suggestion: null,
    itemKey: null,
    itemWhy: null,
    planChanged: null,
    ...patch,
  };
}

/** The items the ANV action can assign. */
export const ITEMS: ItemOption[] = [
  { key: "kz-169", label: "KZ 169 · Digital work equipment", kz: "169" },
  { key: "kz-720", label: "KZ 720 · Professional literature", kz: "720" },
];

/** A demo review queue, with `patch` applied. */
export function reviewQueue(patch: Partial<ReviewQueue> = {}): ReviewQueue {
  return { lines: [reviewLine()], items: ITEMS, subscriptions: [], ...patch };
}
