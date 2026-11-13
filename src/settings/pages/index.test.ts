import { describe, expect, it } from "vitest";
import { groupsOf, searchSettings } from "../registry";
import { SETTINGS_PAGES } from "./index";

const found = (query: string) => searchSettings(SETTINGS_PAGES, query).map((m) => (m.row ? `${m.page.id}/${m.row.id}` : m.page.id));

describe("SETTINGS_PAGES", () => {
  it("puts Accounts and Tax Rules in the Finance group ahead of the App pages", () => {
    const [finance, app] = groupsOf(SETTINGS_PAGES);

    expect(finance.group).toBe("Finance");
    expect(finance.pages.map((p) => p.label)).toEqual(["Accounts", "Tax Rules"]);
    expect(app.group).toBe("App");
  });

  it.each([
    ["Reload after editing", "taxrules/reload"],
    ["IBAN", "accounts/bank-accounts"],
    ["Hide transfers", "accounts/own-transfers"],
    ["Start next year", "taxrules/duplicate"],
    ["Import a JSON file", "taxrules/import"],
  ])("finds the row for %s", (query, hit) => {
    expect(found(query)).toContain(hit);
  });

  it("finds the Finance pages by their group name", () => {
    expect(found("finance")).toEqual(expect.arrayContaining(["accounts", "taxrules"]));
  });
});
