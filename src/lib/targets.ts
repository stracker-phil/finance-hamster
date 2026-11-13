import type { Target } from "../api";
import type { Route } from "../state/route";

/** The page a search match leads to. */
export function routeOf(target: Target): Route {
  switch (target.page) {
    case "documents":
      return { page: "documents" };
    case "document":
      return { page: "document", documentId: target.documentId, year: target.year };
    case "account":
      return { page: "account", accountId: target.accountId };
  }
}
