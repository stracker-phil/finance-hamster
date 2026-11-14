import type { Route } from "../state/route";
import type { HeaderTitle } from "./Header";

/** The header title of a page before its data loads. */
export function titleFor(route: Route): HeaderTitle {
  switch (route.page) {
    case "today":
      return { title: "Today" };
    case "subscriptions":
      return { title: "Subscriptions" };
    case "statements":
      return { title: "Statements" };
    case "documents":
      return { title: "Documents" };
    case "review":
      return { title: "Review", crumb: { label: "Statements", to: { page: "statements" } } };
    case "account":
      return { title: "Account", crumb: { label: "Statements", to: { page: "statements" } } };
    case "year":
      return { title: `Tax year ${route.year}` };
    case "document":
      return {
        title: "Document",
        crumb:
          route.year === null
            ? { label: "Documents", to: { page: "documents" } }
            : { label: `Tax year ${route.year}`, to: { page: "year", year: route.year } },
      };
    case "log":
      return { title: "Log", sub: "Every change to a document, written once", crumb: { label: `Tax year ${route.year}`, to: { page: "year", year: route.year } } };
    case "archive":
      return { title: "Archive", crumb: { label: `Tax year ${route.year}`, to: { page: "year", year: route.year } } };
    case "yearCheck":
      return { title: "Year check", crumb: { label: `Tax year ${route.year}`, to: { page: "year", year: route.year } } };
  }
}
