import type { AttentionKind, DocumentsTab } from "../api";

/** Every page of the main window and what it needs to show. */
export type Route =
  | { page: "today" }
  /** `subscriptionId` opens its panel. */
  | { page: "subscriptions"; subscriptionId?: string }
  | { page: "statements" }
  /** `tab` opens one kind of document; without it, the tab opened last. */
  | { page: "documents"; tab?: DocumentsTab }
  | { page: "statement"; statementId: string }
  | { page: "other"; otherId: string }
  | { page: "review" }
  /** `statementId` scrolls to that statement's lines. */
  | { page: "account"; accountId: string; statementId?: string }
  /** `attention` shows only what needs attention of that kind. */
  | { page: "year"; year: number; attention?: AttentionKind }
  /** `year` is the tax year it was opened from; null when opened from Documents. */
  | { page: "document"; documentId: string; year: number | null }
  | { page: "payslip"; payslipId: string; year: number }
  | { page: "log"; year: number }
  | { page: "archive"; year: number };

export type PageId = Route["page"];

export const HOME: Route = { page: "today" };

/** The sidebar row a page belongs to: a nested page keeps its parent's row selected. */
export function sidebarKey(route: Route): string {
  switch (route.page) {
    case "review":
    case "account":
      return "statements";
    case "document":
      return route.year === null ? "documents" : `year:${route.year}`;
    case "statement":
    case "other":
    case "payslip":
      return "documents";
    case "log":
    case "archive":
    case "year":
      return `year:${route.year}`;
    default:
      return route.page;
  }
}

/** Routes are flat objects; equal when they hold the same keys with the same values, in any order. Absent and
 *  undefined keys count the same. */
export const sameRoute = (a: Route, b: Route) => {
  const entries = (r: Route) => Object.entries(r).filter(([, v]) => v !== undefined);
  const ea = entries(a);
  const fields = new Map(entries(b));
  return ea.length === fields.size && ea.every(([k, v]) => fields.get(k) === v);
};
