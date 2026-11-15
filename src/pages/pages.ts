import { t } from "../i18n";
import type { PageId } from "../state/route";

/** What the header shows for each page: the filter field's placeholder on pages with a list (⌘F), otherwise the
 *  field searches everywhere (⌘K). Read at render time, so each read is in the current language. */
export const FILTER_PLACEHOLDER: Partial<Record<PageId, string>> = {
  get subscriptions() {
    return t().pages.filter.subscriptions;
  },
  get account() {
    return t().pages.filter.account;
  },
  get documents() {
    return t().pages.filter.documents;
  },
  get log() {
    return t().pages.filter.log;
  },
  get statements() {
    return t().pages.filter.statements;
  },
  get year() {
    return t().pages.filter.year;
  },
};

/** Where the header's back arrow leads, and the crumb it shows, for a nested page. */
export const PARENT_LABEL: Partial<Record<PageId, string>> = {
  get account() {
    return t().pages.parent.statements;
  },
  get review() {
    return t().pages.parent.statements;
  },
};
