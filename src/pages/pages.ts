import type { PageId } from "../state/route";

/** What the header shows for each page: the filter field's placeholder on pages with a list (⌘F), otherwise the
 *  field searches everywhere (⌘K). */
export const FILTER_PLACEHOLDER: Partial<Record<PageId, string>> = {
  subscriptions: "Filter subscriptions",
  account: "Filter lines",
  documents: "Filter documents",
  log: "Filter the log",
  statements: "Filter accounts",
  year: "Filter this tax year",
};

/** Where the header's back arrow leads, and the crumb it shows, for a nested page. */
export const PARENT_LABEL: Partial<Record<PageId, string>> = {
  account: "Statements",
  review: "Statements",
};
