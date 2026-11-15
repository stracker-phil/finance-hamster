import { t } from "../i18n";
import type { Route } from "../state/route";
import type { HeaderTitle } from "./Header";

/** The header title of a page before its data loads. */
export function titleFor(route: Route): HeaderTitle {
  const text = t().shell.titles;
  switch (route.page) {
    case "today":
      return { title: text.today };
    case "subscriptions":
      return { title: text.subscriptions };
    case "statements":
      return { title: text.statements };
    case "documents":
      return { title: text.documents };
    case "review":
      return { title: text.review, crumb: { label: text.statements, to: { page: "statements" } } };
    case "account":
      return { title: text.account, crumb: { label: text.statements, to: { page: "statements" } } };
    case "year":
      return { title: text.taxYear(route.year) };
    case "document":
      return {
        title: text.document,
        crumb:
          route.year === null
            ? { label: text.documents, to: { page: "documents" } }
            : { label: text.taxYear(route.year), to: { page: "year", year: route.year } },
      };
    case "payslip":
      return { title: text.payslip, crumb: { label: text.taxYear(route.year), to: { page: "year", year: route.year } } };
    case "log":
      return { title: text.log, sub: text.logSub, crumb: { label: text.taxYear(route.year), to: { page: "year", year: route.year } } };
    case "archive":
      return { title: text.archive, crumb: { label: text.taxYear(route.year), to: { page: "year", year: route.year } } };
  }
}
