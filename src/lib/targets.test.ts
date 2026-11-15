import { describe, expect, it } from "vitest";
import type { Target } from "../api";
import type { Route } from "../state/route";
import { routeOf } from "./targets";

describe("routeOf", () => {
  it.each<[Target, Route]>([
    [{ page: "documents", tab: "invoices" }, { page: "documents", tab: "invoices" }],
    [{ page: "documents", tab: "statements" }, { page: "documents", tab: "statements" }],
    [{ page: "documents", tab: "payslips" }, { page: "documents", tab: "payslips" }],
    [{ page: "documents", tab: "other" }, { page: "documents", tab: "other" }],
    [{ page: "statement", statementId: "st-10" }, { page: "statement", statementId: "st-10" }],
    [{ page: "payslip", payslipId: "ps-7", year: 2026 }, { page: "payslip", payslipId: "ps-7", year: 2026 }],
    [{ page: "other", otherId: "other-1" }, { page: "other", otherId: "other-1" }],
    [{ page: "document", documentId: "doc-9", year: 2026 }, { page: "document", documentId: "doc-9", year: 2026 }],
    [{ page: "document", documentId: "doc-9", year: null }, { page: "document", documentId: "doc-9", year: null }],
    [{ page: "account", accountId: "acc-2" }, { page: "account", accountId: "acc-2" }],
  ])("leads %j to %j", (target, route) => {
    expect(routeOf(target)).toEqual(route);
  });
});
