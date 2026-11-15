import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { setLanguage } from "../i18n";
import { renderWithQuery } from "../test/renderWithQuery";
import { rulesOverview, rulesYear, rulesYearView } from "../test/rules";
import { SettingsWindow } from "./SettingsWindow";

const mocked = vi.mocked(api, true);

beforeEach(() => {
  vi.clearAllMocks();
  setLanguage("en");
  mocked.rulesOverview.mockResolvedValue(rulesOverview([rulesYear()]));
  mocked.rulesYear.mockResolvedValue(rulesYearView());
  mocked.yearCategories.mockResolvedValue([]);
});

const open = () => renderWithQuery(<SettingsWindow request={{ id: "about" }} onOpen={() => {}} />);

describe("SettingsWindow", () => {
  it("shows a page under its label without a back button", () => {
    open();

    expect(screen.getByRole("heading", { level: 2, name: "About" })).toBeTruthy();
    expect(screen.queryByRole("button", { name: /^Back to/ })).toBeNull();
  });

  describe("drilling into a year of Tax Rules", () => {
    it("opens the year when its row is clicked", async () => {
      const onOpen = vi.fn();
      renderWithQuery(<SettingsWindow request={{ id: "taxrules" }} onOpen={onOpen} />);

      await userEvent.click(await screen.findByRole("button", { name: /2026/ }));

      expect(onOpen).toHaveBeenCalledWith("taxrules/2026");
    });

    it("shows the year's title and rules with a back button instead of the page", async () => {
      renderWithQuery(<SettingsWindow request={{ id: "taxrules/2026" }} onOpen={() => {}} />);

      expect(screen.getByRole("heading", { level: 2, name: "Tax Rules 2026" })).toBeTruthy();
      expect(await screen.findByLabelText("Home office limit")).toBeTruthy();
      expect(screen.getByRole("button", { name: "Back to Tax Rules" })).toBeTruthy();
      expect(screen.queryByRole("heading", { level: 3, name: "Tax years" })).toBeNull();
    });

    it("keeps the parent page marked as current in the list", () => {
      renderWithQuery(<SettingsWindow request={{ id: "taxrules/2026" }} onOpen={() => {}} />);

      expect(screen.getByRole("button", { name: "Tax Rules" }).getAttribute("aria-current")).toBe("page");
    });

    it("goes back to the page when the back button is clicked", async () => {
      const onOpen = vi.fn();
      renderWithQuery(<SettingsWindow request={{ id: "taxrules/2026" }} onOpen={onOpen} />);

      await userEvent.click(screen.getByRole("button", { name: "Back to Tax Rules" }));

      expect(onOpen).toHaveBeenCalledWith("taxrules");
    });

    it("shows the year page in German with a German back button", async () => {
      setLanguage("de");
      renderWithQuery(<SettingsWindow request={{ id: "taxrules/2026" }} onOpen={() => {}} />);

      expect(screen.getByRole("heading", { level: 2, name: "Steuerregeln 2026" })).toBeTruthy();
      expect(screen.getByRole("button", { name: "Zurück zu Steuerregeln" })).toBeTruthy();
      await screen.findByLabelText("Home office limit");
    });

    it("falls back to the first page for a detail on a page without details", () => {
      renderWithQuery(<SettingsWindow request={{ id: "about/2026" }} onOpen={() => {}} />);

      expect(screen.queryByRole("button", { name: /^Back to/ })).toBeNull();
      expect(screen.queryByRole("heading", { level: 2, name: /2026/ })).toBeNull();
    });
  });
});

describe("SettingsWindow in German", () => {
  beforeEach(() => setLanguage("de"));

  it("lists the pages under German group and page names", () => {
    open();

    expect(screen.getByRole("navigation", { name: "Einstellungsseiten" })).toBeTruthy();
    expect(screen.getByRole("group", { name: "Finanzen" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Darstellung" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Händler" })).toBeTruthy();
    expect(screen.queryByRole("button", { name: "Appearance" })).toBeNull();
  });

  it.each(["Sprache", "language"])("finds the language row when searching %s", async (query) => {
    open();

    await userEvent.type(screen.getByRole("searchbox", { name: "Einstellungen durchsuchen" }), query);

    expect(screen.getByRole("button", { name: "Sprache, Darstellung" })).toBeTruthy();
  });

  it("still finds a page by its English name", async () => {
    open();

    await userEvent.type(screen.getByRole("searchbox", { name: "Einstellungen durchsuchen" }), "Appearance");

    expect(screen.getByRole("button", { name: "Darstellung" })).toBeTruthy();
  });

  it("says so in German when nothing matches", async () => {
    open();

    await userEvent.type(screen.getByRole("searchbox", { name: "Einstellungen durchsuchen" }), "zzzzzz");

    expect(screen.getByText("Keine Treffer")).toBeTruthy();
  });
});
