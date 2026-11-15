import { describe, expect, it } from "vitest";
import { HOME, sameRoute, sidebarKey, type Route } from "./route";

describe("sameRoute", () => {
  it("is true for equal routes", () => {
    expect(sameRoute({ page: "year", year: 2026 }, { page: "year", year: 2026 })).toBe(true);
    expect(sameRoute(HOME, { page: "today" })).toBe(true);
  });

  it.each<[string, Route, Route]>([
    ["another page", { page: "today" }, { page: "review" }],
    ["another year", { page: "year", year: 2026 }, { page: "year", year: 2025 }],
    ["another attention filter", { page: "year", year: 2026, attention: "draft" }, { page: "year", year: 2026 }],
    ["another subscription panel", { page: "subscriptions", subscriptionId: "sub-1" }, { page: "subscriptions" }],
    ["another account", { page: "account", accountId: "acc-1" }, { page: "account", accountId: "acc-2" }],
    ["another tab of Documents", { page: "documents", tab: "payslips" }, { page: "documents", tab: "other" }],
    ["another statement", { page: "statement", statementId: "st-1" }, { page: "statement", statementId: "st-2" }],
    ["another statement scrolled to in an account", { page: "account", accountId: "acc-1", statementId: "st-1" }, { page: "account", accountId: "acc-1" }],
    ["another origin of a document", { page: "document", documentId: "doc-1", year: 2026 }, { page: "document", documentId: "doc-1", year: null }],
  ])("is false for %s", (_name, a, b) => {
    expect(sameRoute(a, b)).toBe(false);
  });

  it("is true for equal routes whose keys come in another order", () => {
    const a = { page: "year", year: 2026, attention: "draft" } as Route;
    const b = { attention: "draft", year: 2026, page: "year" } as Route;

    expect(sameRoute(a, b)).toBe(true);
    expect(sameRoute(b, a)).toBe(true);
  });

  it("treats an undefined key like an absent one", () => {
    const withUndefined: Route = { page: "year", year: 2026, attention: undefined };
    const without: Route = { page: "year", year: 2026 };

    expect(sameRoute(withUndefined, without)).toBe(true);
    expect(sameRoute(without, withUndefined)).toBe(true);
  });

  it("is false when one route has an extra defined key", () => {
    const extra: Route = { page: "year", year: 2026, attention: "draft" };
    const plain: Route = { page: "year", year: 2026 };

    expect(sameRoute(extra, plain)).toBe(false);
    expect(sameRoute(plain, extra)).toBe(false);
  });
});

describe("sidebarKey", () => {
  it.each<[string, Route, string]>([
    ["today", { page: "today" }, "today"],
    ["subscriptions", { page: "subscriptions", subscriptionId: "sub-1" }, "subscriptions"],
    ["statements", { page: "statements" }, "statements"],
    ["documents", { page: "documents" }, "documents"],
    ["review under Accounts", { page: "review" }, "statements"],
    ["an account under Accounts", { page: "account", accountId: "acc-1" }, "statements"],
    ["a year", { page: "year", year: 2026 }, "year:2026"],
    ["a year's log under its year", { page: "log", year: 2025 }, "year:2025"],
    ["a year's archive under its year", { page: "archive", year: 2025 }, "year:2025"],
    ["a payslip under Documents", { page: "payslip", payslipId: "ps-1", year: 2026 }, "documents"],
    ["a statement under Documents", { page: "statement", statementId: "st-1" }, "documents"],
    ["an other document under Documents", { page: "other", otherId: "other-1" }, "documents"],
    ["a tab of Documents", { page: "documents", tab: "payslips" }, "documents"],
    ["a document opened from a year under that year", { page: "document", documentId: "doc-1", year: 2026 }, "year:2026"],
    ["a document opened from Documents under Documents", { page: "document", documentId: "doc-1", year: null }, "documents"],
  ])("selects the row for %s", (_name, route, key) => {
    expect(sidebarKey(route)).toBe(key);
  });
});
