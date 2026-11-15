import { describe, expect, it } from "vitest";
import { groupsOf, searchSettings } from "../registry";
import { SETTINGS_PAGES } from "./index";

const found = (query: string) => searchSettings(SETTINGS_PAGES, query).map((m) => (m.row ? `${m.page.id}/${m.row.id}` : m.page.id));

describe("SETTINGS_PAGES", () => {
  it("puts Rules, Vendors, Accounts and Tax Rules in the Finance group ahead of the App pages", () => {
    const [finance, app] = groupsOf(SETTINGS_PAGES);

    expect(finance.group).toBe("Finance");
    expect(finance.pages.map((p) => p.label)).toEqual(["Rules", "Vendors", "Accounts", "Tax Rules"]);
    expect(app.group).toBe("App");
  });

  it.each([
    ["New rule", "rules/add-rule"],
    ["Apply rules again", "rules/rerun"],
    ["bank text", "rules/add-rule"],
    ["rerun", "rules/rerun"],
  ])("finds the Rules row for %s", (query, hit) => {
    expect(found(query)).toContain(hit);
  });

  it.each([
    ["Tax years", "taxrules/tax-years"],
    ["Kennzahl", "taxrules/tax-years"],
    ["JSON", "taxrules/tax-years"],
    ["IBAN", "accounts/bank-accounts"],
    ["Hide transfers", "accounts/own-transfers"],
    ["Start next year", "taxrules/start-year"],
    ["Steuerjahre", "taxrules/tax-years"],
    ["merge", "vendors/vendor-list"],
    ["default item", "vendors/vendor-list"],
    ["Days in Archive", "privacy/archive"],
    ["exclude", "privacy/archive"],
    ["Deletion log", "privacy/deletion-log"],
    ["Verify files", "privacy/verify-files"],
    ["checksum", "privacy/verify-files"],
    ["Time Machine", "privacy/backup"],
    ["backup", "privacy/backup"],
    ["rebind", "shortcuts/commands"],
    ["keyboard", "shortcuts/commands"],
    ["In every window", "shortcuts/commands"],
    ["On the Review page", "shortcuts/review-keys"],
    ["Review keys", "shortcuts/review-keys"],
    ["skip", "shortcuts/review-keys"],
    ["On the Year Check page", "shortcuts/year-check-keys"],
    ["later", "shortcuts/year-check-keys"],
    ["Data folder", "about/about-data-folder"],
    ["finder", "about/about-data-folder"],
    ["Log folder", "about/about-log-folder"],
    ["diagnostics", "about/about-log-folder"],
    ["Statement chains", "status/status-statements"],
    ["Files and journal", "status/status-files"],
    ["Rules files", "status/status-rules"],
  ])("finds the row for %s", (query, hit) => {
    expect(found(query)).toContain(hit);
  });

  it.each([
    ["Default reminder", "reminders/default-reminder"],
    ["renewal", "reminders/default-reminder"],
    ["price changes", "reminders/price-changes"],
    ["Remind me to import", "reminders/import-after"],
    ["overdue", "reminders/import-after"],
    ["gaps", "reminders/gaps"],
    ["Filing reminder", "reminders/filing"],
    ["november", "reminders/year-check"],
  ])("finds the Reminders row for %s", (query, hit) => {
    expect(found(query)).toContain(hit);
  });

  it.each(["sprache", "deutsch", "german", "englisch"])("finds the Appearance language row for %s", (query) => {
    expect(found(query)).toContain("appearance/language");
  });

  it("finds the Finance pages by their group name", () => {
    expect(found("finance")).toEqual(expect.arrayContaining(["rules", "vendors", "accounts", "taxrules"]));
  });
});
