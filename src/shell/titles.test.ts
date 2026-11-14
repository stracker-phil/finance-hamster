import { describe, expect, it } from "vitest";
import { titleFor } from "./titles";

describe("titleFor", () => {
  it("titles the year check with a crumb back to its tax year", () => {
    expect(titleFor({ page: "yearCheck", year: 2026 })).toEqual({
      title: "Year check",
      crumb: { label: "Tax year 2026", to: { page: "year", year: 2026 } },
    });
  });
});
