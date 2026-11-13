import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../../test/apiMock")).createApiMock() }));

import { api } from "../../api";
import { account } from "../../test/accounts";
import { renderWithQuery } from "../../test/renderWithQuery";
import { settings } from "../../test/settings";
import { AccountsPage } from "./AccountsPage";

const mocked = vi.mocked(api, true);

beforeEach(() => {
  vi.clearAllMocks();
  mocked.listAccounts.mockResolvedValue([account()]);
  mocked.getSettings.mockResolvedValue(settings({ hideOwnTransfers: true }));
  mocked.updateSettings.mockResolvedValue(settings());
});

describe("AccountsPage", () => {
  describe("account list", () => {
    it("lists each account with its bank and masked IBAN", async () => {
      mocked.listAccounts.mockResolvedValue([account(), account({ id: "acc-2", name: "Savings", bank: "Sample Bank", ibanMasked: "AT•• •••• 0002" })]);
      renderWithQuery(<AccountsPage />);

      expect(await screen.findByText("Private")).toBeTruthy();
      expect(screen.getByText("Example Bank · AT•• •••• 0001")).toBeTruthy();
      expect(screen.getByText("Savings")).toBeTruthy();
      expect(screen.getByText("Sample Bank · AT•• •••• 0002")).toBeTruthy();
    });

    it("leaves out the bank separator when the bank is empty", async () => {
      mocked.listAccounts.mockResolvedValue([account({ bank: "" })]);
      renderWithQuery(<AccountsPage />);

      expect(await screen.findByText("AT•• •••• 0001")).toBeTruthy();
    });

    it("says there are no accounts yet when the list is empty", async () => {
      mocked.listAccounts.mockResolvedValue([]);
      renderWithQuery(<AccountsPage />);

      expect(await screen.findByText("No accounts yet")).toBeTruthy();
      expect(screen.queryByRole("button", { name: "Edit…" })).toBeNull();
    });
  });

  describe("sheets", () => {
    it("opens the edit sheet of the account whose Edit… is clicked", async () => {
      mocked.listAccounts.mockResolvedValue([account(), account({ id: "acc-2", name: "Savings" })]);
      renderWithQuery(<AccountsPage />);
      await screen.findByText("Savings");

      await userEvent.click(screen.getAllByRole("button", { name: "Edit…" })[1]);

      expect(screen.getByRole("dialog", { name: "Edit Account" })).toBeTruthy();
      expect(screen.getByLabelText<HTMLInputElement>("Name").value).toBe("Savings");
    });

    it("opens an empty add sheet from Add Account…", async () => {
      renderWithQuery(<AccountsPage />);

      await userEvent.click(screen.getByRole("button", { name: "Add Account…" }));

      expect(screen.getByRole("dialog", { name: "Add Account" })).toBeTruthy();
      expect(screen.getByLabelText<HTMLInputElement>("Name").value).toBe("");
    });

    it("closes the sheet on Cancel", async () => {
      renderWithQuery(<AccountsPage />);
      await userEvent.click(screen.getByRole("button", { name: "Add Account…" }));

      await userEvent.click(screen.getByRole("button", { name: "Cancel" }));

      expect(screen.queryByRole("dialog")).toBeNull();
    });
  });

  describe("hide transfers switch", () => {
    const toggle = () => screen.getByRole("switch", { name: "Hide transfers between my accounts" });

    it("shows the saved value", async () => {
      mocked.getSettings.mockResolvedValue(settings({ hideOwnTransfers: false }));
      renderWithQuery(<AccountsPage />);

      await waitFor(() => expect(toggle()).not.toBeChecked());
    });

    it("saves turning it off", async () => {
      renderWithQuery(<AccountsPage />);
      await waitFor(() => expect(toggle()).toBeChecked());

      await userEvent.click(toggle());

      expect(mocked.updateSettings).toHaveBeenCalledWith({ hideOwnTransfers: false });
    });
  });
});
