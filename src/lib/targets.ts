import type { Target } from "../api";
import type { Route } from "../state/route";

/** The page a search match leads to. */
export function routeOf(target: Target): Route {
  switch (target.page) {
    case "documents":
      return { page: "documents", tab: target.tab };
    case "statement":
      return { page: "statement", statementId: target.statementId };
    case "payslip":
      return { page: "payslip", payslipId: target.payslipId, year: target.year };
    case "other":
      return { page: "other", otherId: target.otherId };
    case "document":
      return { page: "document", documentId: target.documentId, year: target.year };
    case "account":
      return { page: "account", accountId: target.accountId };
  }
}
