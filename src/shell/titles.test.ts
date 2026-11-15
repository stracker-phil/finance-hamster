import { describe, expect, it } from "vitest";
import { titleFor } from "./titles";

describe("titleFor", () => {
  it("titles the tax year with no crumb", () => {
    expect(titleFor({ page: "year", year: 2026 })).toEqual({ title: "Tax year 2026" });
  });

  it("titles a document opened from a year with a crumb back to that year", () => {
    expect(titleFor({ page: "document", documentId: "doc-1", year: 2026 })).toEqual({
      title: "Document",
      crumb: { label: "Tax year 2026", to: { page: "year", year: 2026 } },
    });
  });

  it("titles a document opened from Documents with a crumb back to Documents", () => {
    expect(titleFor({ page: "document", documentId: "doc-1", year: null }).crumb).toEqual({ label: "Documents", to: { page: "documents" } });
  });

  it.each([
    ["a payslip", { page: "payslip", payslipId: "ps-1", year: 2026 }, "Payslip", "payslips"],
    ["a statement", { page: "statement", statementId: "st-1" }, "Statement", "statements"],
    ["an other document", { page: "other", otherId: "other-1" }, "Document", "other"],
  ] as const)("titles %s with a crumb back to its tab of Documents", (_name, route, title, tab) => {
    expect(titleFor(route)).toEqual({ title, crumb: { label: "Documents", to: { page: "documents", tab } } });
  });

  it.each(["review", "account"] as const)("titles the %s page with a crumb back to Accounts", (page) => {
    const route = page === "review" ? ({ page } as const) : ({ page, accountId: "acc-1" } as const);

    expect(titleFor(route).crumb).toEqual({ label: "Accounts", to: { page: "statements" } });
  });

  it("calls the coverage page Accounts", () => {
    expect(titleFor({ page: "statements" })).toEqual({ title: "Accounts" });
  });
});
