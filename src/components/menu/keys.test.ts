import { describe, expect, it } from "vitest";
import { nextEnabled, rowStartingWith } from "./keys";
import type { MenuEntry } from "./types";

describe("nextEnabled", () => {
  const enabled = [1, 3, 4];

  it("steps to the next enabled row down and up", () => {
    expect(nextEnabled(enabled, 1, 1)).toBe(3);
    expect(nextEnabled(enabled, 4, -1)).toBe(3);
  });

  it("wraps from the last row to the first and back", () => {
    expect(nextEnabled(enabled, 4, 1)).toBe(1);
    expect(nextEnabled(enabled, 1, -1)).toBe(4);
  });

  it("starts at the first row going down and at the last going up when none is active", () => {
    expect(nextEnabled(enabled, -1, 1)).toBe(1);
    expect(nextEnabled(enabled, -1, -1)).toBe(4);
  });

  it("stays where it is when no row is enabled", () => {
    expect(nextEnabled([], 2, 1)).toBe(2);
  });
});

describe("rowStartingWith", () => {
  const entries: MenuEntry[] = [{ text: "Archive" }, "separator", { text: "Log" }, { text: "Export" }, { text: "Archived items", enabled: false }, { text: "Edit" }];
  const enabled = [0, 2, 3, 5];

  it("finds the next row after the active one that starts with the letter", () => {
    expect(rowStartingWith(entries, enabled, 3, "e")).toBe(5);
  });

  it("wraps around to rows before the active one", () => {
    expect(rowStartingWith(entries, enabled, 5, "e")).toBe(3);
    expect(rowStartingWith(entries, enabled, 2, "a")).toBe(0);
  });

  it("ignores case", () => {
    expect(rowStartingWith(entries, enabled, 0, "L")).toBe(2);
  });

  it("skips disabled rows", () => {
    expect(rowStartingWith(entries, enabled, 0, "a")).toBe(0);
  });

  it("returns undefined when no enabled row starts with the letter", () => {
    expect(rowStartingWith(entries, enabled, 0, "z")).toBeUndefined();
  });

  it("starts from the first row when none is active", () => {
    expect(rowStartingWith(entries, enabled, -1, "e")).toBe(3);
  });
});
