import { act, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../../test/apiMock")).createApiMock() }));

import { api } from "../../api";
import { createQueryClient } from "../../queries/queryClient";
import { useStore } from "../../state/store";
import { SETTINGS_PAGES } from "../../settings/pages";
import { account } from "../../test/accounts";
import { settings } from "../../test/settings";
import { SettingsApp } from "./SettingsApp";

const mocked = vi.mocked(api, true);
const FIRST_PAGE = SETTINGS_PAGES[0].label;

const renderSettings = () => render(<SettingsApp client={createQueryClient()} />);
const pageTitle = () => screen.getByRole("heading", { level: 2 }).textContent;
const nav = () => within(screen.getByRole("navigation", { name: "Settings pages" }));
const search = () => screen.getByRole("searchbox", { name: "Search settings" });

async function emitOpenSettings(id: string) {
  await waitFor(() => expect(mocked.listen.mock.calls.some(([name]) => name === "open_settings")).toBe(true));
  const handler = mocked.listen.mock.calls.find(([name]) => name === "open_settings")![1] as (id: string) => void;
  act(() => handler(id));
}

describe("SettingsApp", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    window.history.replaceState(null, "", "/");
    useStore.setState({ toasts: [], confirmation: null });
    mocked.listen.mockResolvedValue(() => {});
    mocked.getSettings.mockResolvedValue(settings());
    mocked.listAccounts.mockResolvedValue([account()]);
    mocked.listCommands.mockResolvedValue([]);
    delete document.documentElement.dataset.scheme;
    delete document.documentElement.dataset.accent;
  });

  describe("navigation", () => {
    it("lists the pages under the App heading", () => {
      renderSettings();

      const group = within(screen.getByRole("group", { name: "App" }));
      const labels = group.getAllByRole("button").map((b) => b.textContent);
      expect(labels).toEqual(["Privacy & Data", "Appearance", "Shortcuts", "Status", "About"]);
    });

    it("shows the first page by default", () => {
      renderSettings();

      expect(pageTitle()).toBe(FIRST_PAGE);
      expect(nav().getByRole("button", { name: FIRST_PAGE }).getAttribute("aria-current")).toBe("page");
    });

    it("shows Rules as the first page, in the Finance group", () => {
      renderSettings();

      expect(pageTitle()).toBe("Rules");
      const labels = within(screen.getByRole("group", { name: "Finance" })).getAllByRole("button").map((b) => b.textContent);
      expect(labels).toEqual(["Rules", "Vendors", "Accounts", "Tax Rules"]);
    });

    it("asks for confirmation in the Settings window when Re-run… is clicked", async () => {
      renderSettings();

      await userEvent.click(await screen.findByRole("button", { name: "Re-run…" }));

      expect(screen.getByRole("alertdialog", { name: "Apply the rules again?" })).toBeTruthy();
    });

    it("opens the page named by ?page= in the location", () => {
      window.history.replaceState(null, "", "/?page=appearance");

      renderSettings();

      expect(pageTitle()).toBe("Appearance");
    });

    it("opens the page that holds the row named by ?page=", () => {
      window.history.replaceState(null, "", "/?page=number-format");

      renderSettings();

      expect(pageTitle()).toBe("Appearance");
    });

    it("opens a page when its nav row is clicked", async () => {
      renderSettings();

      await userEvent.click(nav().getByRole("button", { name: "Shortcuts" }));

      expect(pageTitle()).toBe("Shortcuts");
      expect(nav().getByRole("button", { name: "Shortcuts" }).getAttribute("aria-current")).toBe("page");
    });

    it("switches page when an open_settings event arrives", async () => {
      renderSettings();

      await emitOpenSettings("about");

      expect(pageTitle()).toBe("About");
    });
  });

  describe("search", () => {
    it("lists the pages and rows that match, naming a row with its page", async () => {
      renderSettings();

      await userEvent.type(search(), "dark");

      expect(nav().getAllByRole("button").map((b) => b.getAttribute("aria-label"))).toEqual(["Mode, Appearance"]);
    });

    it("opens the page of a result when it is clicked", async () => {
      renderSettings();
      await userEvent.type(search(), "dark");

      await userEvent.click(nav().getByRole("button", { name: "Mode, Appearance" }));

      expect(pageTitle()).toBe("Appearance");
    });

    it("opens the first result on Enter", async () => {
      renderSettings();

      await userEvent.type(search(), "dark{Enter}");

      expect(pageTitle()).toBe("Appearance");
    });

    it("says No Results when nothing matches and Enter opens nothing", async () => {
      renderSettings();

      await userEvent.type(search(), "zzzz{Enter}");

      expect(screen.getByText("No Results")).toBeTruthy();
      expect(pageTitle()).toBe(FIRST_PAGE);
    });

    it("brings back the page list when the search is cleared", async () => {
      renderSettings();
      await userEvent.type(search(), "dark");

      await userEvent.clear(search());

      expect(nav().getByRole("button", { name: "Status" })).toBeTruthy();
    });
  });

  describe("look", () => {
    it("applies the saved appearance and accent to the window", async () => {
      mocked.getSettings.mockResolvedValue(settings({ appearance: "dark", accent: "teal" }));

      renderSettings();

      await waitFor(() => expect(document.documentElement.dataset.scheme).toBe("dark"));
      expect(document.documentElement.dataset.accent).toBe("teal");
    });

    it("applies the new accent when the settings changed event fires", async () => {
      renderSettings();
      await waitFor(() => expect(document.documentElement.dataset.accent).toBe("system"));

      mocked.getSettings.mockResolvedValue(settings({ accent: "red" }));
      await waitFor(() => expect(mocked.listen.mock.calls.some(([name]) => name === "settings_changed")).toBe(true));
      const handler = mocked.listen.mock.calls.find(([name]) => name === "settings_changed")![1] as (payload: null) => void;
      act(() => handler(null));

      await waitFor(() => expect(document.documentElement.dataset.accent).toBe("red"));
    });
  });
});
