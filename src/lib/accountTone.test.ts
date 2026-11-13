import { describe, expect, it } from "vitest";
import { accountTone } from "./accountTone";

describe("accountTone", () => {
  it.each([
    [0, "acct-0"],
    [5, "acct-5"],
    [6, "acct-0"],
    [8, "acct-2"],
  ])("gives account %i the tone %s", (index, tone) => {
    expect(accountTone(index)).toBe(tone);
  });
});
