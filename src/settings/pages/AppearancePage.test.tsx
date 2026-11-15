import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../../test/apiMock")).createApiMock() }));

import { api } from "../../api";
import { setLanguage } from "../../i18n";
import { useStore } from "../../state/store";
import { renderWithQuery } from "../../test/renderWithQuery";
import { settings } from "../../test/settings";
import { AppearancePage } from "./AppearancePage";

const mocked = vi.mocked(api, true);

const loaded = (name: string) => waitFor(() => expect(screen.getByRole("button", { name }).getAttribute("aria-pressed")).toBe("true"));

describe("AppearancePage", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useStore.setState({ toasts: [] });
    mocked.getSettings.mockResolvedValue(settings({ appearance: "light", accent: "blue", numberFormat: "comma_decimal", showOriginalCurrency: false }));
    mocked.updateSettings.mockResolvedValue(settings());
  });

  describe("language", () => {
    it("presses the saved language", async () => {
      mocked.getSettings.mockResolvedValue(settings({ language: "de" }));
      renderWithQuery(<AppearancePage />);

      await loaded("Deutsch");
      expect(screen.getByRole("button", { name: "English" }).getAttribute("aria-pressed")).toBe("false");
    });

    it.each([
      ["Deutsch", "de"],
      ["English", "en"],
    ])("saves %s as the language", async (name, language) => {
      mocked.getSettings.mockResolvedValue(settings({ language: language === "de" ? "en" : "de" }));
      renderWithQuery(<AppearancePage />);
      await loaded(language === "de" ? "English" : "Deutsch");

      await userEvent.click(screen.getByRole("button", { name }));

      expect(mocked.updateSettings).toHaveBeenCalledWith({ language });
    });

    it("reads Sprache for the group and the row in German", () => {
      setLanguage("de");

      renderWithQuery(<AppearancePage />);

      expect(screen.getByRole("heading", { name: "Sprache" })).toBeTruthy();
      expect(screen.getByRole("group", { name: "Sprache" })).toBeTruthy();
      expect(screen.queryByText("Language")).toBeNull();
    });
  });

  describe("mode", () => {
    it("presses the saved mode", async () => {
      renderWithQuery(<AppearancePage />);

      await loaded("Light");
      expect(screen.getByRole("button", { name: "Dark" }).getAttribute("aria-pressed")).toBe("false");
      expect(screen.getByRole("button", { name: "Automatic" }).getAttribute("aria-pressed")).toBe("false");
    });

    it.each([
      ["Automatic", "system"],
      ["Dark", "dark"],
    ])("saves %s as the appearance", async (name, appearance) => {
      renderWithQuery(<AppearancePage />);
      await loaded("Light");

      await userEvent.click(screen.getByRole("button", { name }));

      expect(mocked.updateSettings).toHaveBeenCalledWith({ appearance });
    });
  });

  describe("accent", () => {
    const accents = () => within(screen.getByRole("radiogroup", { name: "Accent" }));

    it("offers ten accents and checks the saved one", async () => {
      renderWithQuery(<AppearancePage />);

      expect(accents().getAllByRole("radio")).toHaveLength(10);
      await waitFor(() => expect(accents().getByRole("radio", { name: "Blue" }).getAttribute("aria-checked")).toBe("true"));
      expect(accents().getByRole("radio", { name: "Red" }).getAttribute("aria-checked")).toBe("false");
    });

    it("saves the clicked accent", async () => {
      renderWithQuery(<AppearancePage />);

      await userEvent.click(accents().getByRole("radio", { name: "Teal" }));

      expect(mocked.updateSettings).toHaveBeenCalledWith({ accent: "teal" });
    });
  });

  describe("amounts", () => {
    it("presses the saved number format and saves the other one", async () => {
      renderWithQuery(<AppearancePage />);
      await loaded("1.234,56");

      await userEvent.click(screen.getByRole("button", { name: "1,234.56" }));

      expect(mocked.updateSettings).toHaveBeenCalledWith({ numberFormat: "dot_decimal" });
    });

    it("shows the foreign currency switch off when saved off and saves turning it on", async () => {
      renderWithQuery(<AppearancePage />);
      const toggle = screen.getByRole("switch", { name: "Show foreign currency next to amounts" });
      await waitFor(() => expect(toggle).not.toBeChecked());

      await userEvent.click(toggle);

      expect(mocked.updateSettings).toHaveBeenCalledWith({ showOriginalCurrency: true });
    });

    it("keeps the foreign currency switch on until the settings load", () => {
      mocked.getSettings.mockReturnValue(new Promise(() => {}));

      renderWithQuery(<AppearancePage />);

      expect(screen.getByRole("switch", { name: "Show foreign currency next to amounts" })).toBeChecked();
    });
  });

  it("shows the error as a toast when a change cannot be saved", async () => {
    mocked.updateSettings.mockRejectedValue({ kind: "io", message: "Disk full", code: null });
    renderWithQuery(<AppearancePage />);

    await userEvent.click(screen.getByRole("button", { name: "Dark" }));

    await waitFor(() => expect(useStore.getState().toasts.map((t) => t.message)).toEqual(["Disk full"]));
  });
});
