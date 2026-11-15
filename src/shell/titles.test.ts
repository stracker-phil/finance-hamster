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

  it("titles a payslip with a crumb back to its tax year", () => {
    expect(titleFor({ page: "payslip", payslipId: "ps-1", year: 2026 })).toEqual({
      title: "Payslip",
      crumb: { label: "Tax year 2026", to: { page: "year", year: 2026 } },
    });
  });

  it("titles a document opened from Documents with a crumb back to Documents", () => {
    expect(titleFor({ page: "document", documentId: "doc-1", year: null }).crumb).toEqual({ label: "Documents", to: { page: "documents" } });
  });
});
