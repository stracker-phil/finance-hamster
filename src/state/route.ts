/** Every page of the main window and what it needs to show. */
export type Route =
  | { page: "today" }
  | { page: "subscriptions" }
  | { page: "statements" }
  | { page: "documents" }
  | { page: "review" }
  | { page: "account"; accountId: string }
  | { page: "year"; year: number }
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

export const sameRoute = (a: Route, b: Route) => JSON.stringify(a) === JSON.stringify(b);
