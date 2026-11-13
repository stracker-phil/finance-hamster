import { describe, expect, it } from "vitest";
import { api } from ".";

describe("api.pdfPageUrl", () => {
  it("builds the pdf-page URL with an encoded id and a rounded width", () => {
    expect(api.pdfPageUrl({ kind: "statement", id: "a b" }, 2, 300.4)).toBe("pdf-page://localhost/statement/a%20b/2.png?w=300");
  });

  it("rounds the width up from half", () => {
    expect(api.pdfPageUrl({ kind: "document", id: "d1" }, 1, 299.5)).toBe("pdf-page://localhost/document/d1/1.png?w=300");
  });
});
