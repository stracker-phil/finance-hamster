import { fireEvent, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../../test/apiMock")).createApiMock() }));

import { api } from "../../api";
import { renderWithQuery } from "../../test/renderWithQuery";
import { settings } from "../../test/settings";
import { RemindersPage } from "./RemindersPage";

const mocked = vi.mocked(api, true);

describe("RemindersPage", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocked.getSettings.mockResolvedValue(settings());
    mocked.updateSettings.mockResolvedValue(settings());
  });

  it("groups the rows under Subscriptions, Statements and Tax year", () => {
    renderWithQuery(<RemindersPage />);

    expect(screen.getByText("Subscriptions")).toBeTruthy();
    expect(screen.getByText("Statements")).toBeTruthy();
    expect(screen.getByText("Tax year")).toBeTruthy();
  });

  describe("default reminder", () => {
    it("marks the current value pressed", async () => {
      mocked.getSettings.mockResolvedValue(settings({ defaultReminder: "one_day" }));
      renderWithQuery(<RemindersPage />);

      await waitFor(() => expect(screen.getByRole("button", { name: "1 day" }).getAttribute("aria-pressed")).toBe("true"));
      expect(screen.getByRole("button", { name: "1 week" }).getAttribute("aria-pressed")).toBe("false");
    });

    it("saves the chosen value", async () => {
      renderWithQuery(<RemindersPage />);

      await userEvent.click(screen.getByRole("button", { name: "1 week" }));

      await waitFor(() => expect(mocked.updateSettings).toHaveBeenCalledWith({ defaultReminder: "one_week" }));
    });
  });

  describe.each([
    ["Tell me when a price changes", "remindPriceChanges"],
    ["Tell me about gaps", "remindGaps"],
    ["Filing reminder", "remindFiling"],
    ["Year questions", "remindQuestions"],
  ] as const)("the %s switch", (label, field) => {
    it("reflects its setting", async () => {
      mocked.getSettings.mockResolvedValue(settings({ [field]: false }));
      renderWithQuery(<RemindersPage />);

      await waitFor(() => expect(screen.getByRole<HTMLInputElement>("switch", { name: new RegExp(label) }).checked).toBe(false));
    });

    it("turns the setting off when it is on", async () => {
      renderWithQuery(<RemindersPage />);

      await userEvent.click(screen.getByRole("switch", { name: new RegExp(label) }));

      await waitFor(() => expect(mocked.updateSettings).toHaveBeenCalledWith({ [field]: false }));
    });

    it("turns the setting on when it is off", async () => {
      mocked.getSettings.mockResolvedValue(settings({ [field]: false }));
      renderWithQuery(<RemindersPage />);
      await waitFor(() => expect(screen.getByRole<HTMLInputElement>("switch", { name: new RegExp(label) }).checked).toBe(false));

      await userEvent.click(screen.getByRole("switch", { name: new RegExp(label) }));

      await waitFor(() => expect(mocked.updateSettings).toHaveBeenCalledWith({ [field]: true }));
    });
  });

  describe("remind me to import after", () => {
    it("shows the configured days", async () => {
      renderWithQuery(<RemindersPage />);

      await waitFor(() => expect(screen.getByLabelText<HTMLInputElement>("Remind me to import after").value).toBe("30"));
    });

    it("saves a valid number of days", async () => {
      renderWithQuery(<RemindersPage />);

      fireEvent.change(screen.getByLabelText("Remind me to import after"), { target: { value: "45" } });

      await waitFor(() => expect(mocked.updateSettings).toHaveBeenCalledWith({ importAfterDays: 45 }));
    });

    it.each(["0", "-3", ""])("ignores %j", (value) => {
      renderWithQuery(<RemindersPage />);

      fireEvent.change(screen.getByLabelText("Remind me to import after"), { target: { value } });

      expect(mocked.updateSettings).not.toHaveBeenCalled();
    });
  });
});
