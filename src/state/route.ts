import type { AttentionKind } from "../api";

/** Every page of the main window and what it needs to show. */
export type Route =
  | { page: "today" }
  /** `subscriptionId` opens its panel. */
  | { page: "subscriptions"; subscriptionId?: string }
  | { page: "statements" }
  | { page: "documents" }
  | { page: "review" }
  | { page: "account"; accountId: string }
  /** `attention` shows only what needs attention of that kind. */
  | { page: "year"; year: number; attention?: AttentionKind }
  /** `year` is the tax year it was opened from; null when opened from Documents. */
  | { page: "document"; documentId: string; year: number | null }
  | { page: "log"; year: number }
  | { page: "archive"; year: number }
  | { page: "yearCheck"; year: number };

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
    case "log":
    case "archive":
    case "yearCheck":
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
