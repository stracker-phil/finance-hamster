import { describe, expect, it } from "vitest";
import { dayMonthYear } from "./date";

describe("dayMonthYear", () => {
  it.each([
    [new Date(2026, 10, 13), "13 Nov 2026"],
    [new Date(2026, 0, 5), "5 Jan 2026"],
    [new Date(2026, 11, 31), "31 Dec 2026"],
  ])("writes %s day first with a short month", (day, text) => {
    expect(dayMonthYear(day)).toBe(text);
  });
});
