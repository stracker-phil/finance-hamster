import { fireEvent, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../../test/apiMock")).createApiMock() }));

import { api } from "../../api";
import { useStore } from "../../state/store";
import { deletionView } from "../../test/documents";
import { fileCheck, fileProblem } from "../../test/integrity";
import { renderWithQuery } from "../../test/renderWithQuery";
import { settings } from "../../test/settings";
import { PrivacyPage } from "./PrivacyPage";

const mocked = vi.mocked(api, true);

describe("PrivacyPage", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useStore.setState({ toasts: [] });
    mocked.dataFolder.mockResolvedValue("/Users/demo/Library/Application Support/com.example.hamster/");
    mocked.showDataFolder.mockResolvedValue(undefined);
    mocked.getSettings.mockResolvedValue(settings());
    mocked.updateSettings.mockResolvedValue(settings());
    mocked.deletionLog.mockResolvedValue([]);
    mocked.filesLastCheck.mockResolvedValue(null);
    mocked.filesVerify.mockResolvedValue(fileCheck());
  });

  describe("Verify files", () => {
    it("explains the check and says it has not run since launch", async () => {
      renderWithQuery(<PrivacyPage />);

      expect(await screen.findByText(/Compares every PDF with its stored checksum\./)).toBeTruthy();
      expect(screen.getByText(/Not run since launch\./)).toBeTruthy();
    });

    it("reports that all files match after a clean run", async () => {
      mocked.filesLastCheck.mockResolvedValue(fileCheck({ checked: 12 }));
      renderWithQuery(<PrivacyPage />);

      expect(await screen.findByText(/Last run: .+ · all 12 files match\./)).toBeTruthy();
      expect(screen.queryByText("Missing")).toBeNull();
      expect(screen.queryByText("Changed")).toBeNull();
    });

    it("reports how many files are missing or changed and lists each problem", async () => {
      mocked.filesLastCheck.mockResolvedValue(
        fileCheck({
          checked: 12,
          problems: [
            fileProblem(),
            fileProblem({ trouble: "changed", file: "docs/inbox/statement-010-2026.pdf", what: "Statement · Private Nr. 010/2026" }),
          ],
        }),
      );
      renderWithQuery(<PrivacyPage />);

      expect(await screen.findByText(/Last run: .+ · 2 of 12 files missing or changed\./)).toBeTruthy();
      expect(screen.getByText("Document · Demo Cloud Ltd invoice")).toBeTruthy();
      expect(screen.getByText("docs/2026/2026-11-09 Demo Cloud Ltd.pdf")).toBeTruthy();
      expect(screen.getByText("Missing")).toBeTruthy();
      expect(screen.getByText("Statement · Private Nr. 010/2026")).toBeTruthy();
      expect(screen.getByText("docs/inbox/statement-010-2026.pdf")).toBeTruthy();
      expect(screen.getByText("Changed")).toBeTruthy();
    });

    it("runs the check on Verify Now and stays disabled until it finishes", async () => {
      let finish: (check: ReturnType<typeof fileCheck>) => void = () => {};
      mocked.filesVerify.mockReturnValue(new Promise((resolve) => (finish = resolve)));
      renderWithQuery(<PrivacyPage />);

      await userEvent.click(screen.getByRole("button", { name: "Verify Now" }));

      expect(mocked.filesVerify).toHaveBeenCalled();
      expect(screen.getByRole("button", { name: "Verify Now" })).toBeDisabled();
      finish(fileCheck());
      await waitFor(() => expect(screen.getByRole("button", { name: "Verify Now" })).toBeEnabled());
    });

    it("shows the error as a toast when the check fails, and enables the button again", async () => {
      mocked.filesVerify.mockRejectedValue({ kind: "io", message: "Data folder unreadable", code: null });
      renderWithQuery(<PrivacyPage />);

      await userEvent.click(screen.getByRole("button", { name: "Verify Now" }));

      await waitFor(() => expect(useStore.getState().toasts.map((t) => t.message)).toEqual(["Data folder unreadable"]));
      expect(screen.getByRole("button", { name: "Verify Now" })).toBeEnabled();
    });
  });

  it("shows the data folder path", async () => {
    renderWithQuery(<PrivacyPage />);

    expect(await screen.findByText("/Users/demo/Library/Application Support/com.example.hamster/")).toBeTruthy();
  });

  it("reveals the data folder in Finder", async () => {
    renderWithQuery(<PrivacyPage />);

    await userEvent.click(screen.getByRole("button", { name: "Show in Finder" }));

    expect(mocked.showDataFolder).toHaveBeenCalled();
  });

  it("shows the error as a toast when the folder cannot be shown", async () => {
    mocked.showDataFolder.mockRejectedValue({ kind: "not_found", message: "The folder is gone", code: null });
    renderWithQuery(<PrivacyPage />);

    await userEvent.click(screen.getByRole("button", { name: "Show in Finder" }));

    await waitFor(() => expect(useStore.getState().toasts.map((t) => t.message)).toEqual(["The folder is gone"]));
  });

  describe("days in Archive before delete", () => {
    it("shows the configured days", async () => {
      mocked.getSettings.mockResolvedValue(settings({ archiveDays: 14 }));
      renderWithQuery(<PrivacyPage />);

      await waitFor(() => expect(screen.getByLabelText<HTMLInputElement>("Days in Archive before delete").value).toBe("14"));
    });

    it("saves a valid number of days", async () => {
      renderWithQuery(<PrivacyPage />);

      fireEvent.change(screen.getByLabelText("Days in Archive before delete"), { target: { value: "30" } });

      await waitFor(() => expect(mocked.updateSettings).toHaveBeenCalledWith({ archiveDays: 30 }));
    });

    it.each(["0", "-3", ""])("ignores %j", (value) => {
      renderWithQuery(<PrivacyPage />);

      fireEvent.change(screen.getByLabelText("Days in Archive before delete"), { target: { value: "5" } });
      mocked.updateSettings.mockClear();
      fireEvent.change(screen.getByLabelText("Days in Archive before delete"), { target: { value } });

      expect(mocked.updateSettings).not.toHaveBeenCalled();
    });
  });

  describe("deletion log", () => {
    it("lists each deletion with its file, dates and reason", async () => {
      mocked.deletionLog.mockResolvedValue([deletionView()]);
      renderWithQuery(<PrivacyPage />);

      expect(await screen.findByText("Sample Books")).toBeTruthy();
      expect(screen.getByText("sample-books.pdf · Added 2 Mar 2026 · deleted 13 Nov 2026 · Scanned twice")).toBeTruthy();
      expect(screen.queryByText("Nothing deleted")).toBeNull();
    });

    it("says nothing was deleted when the log is empty", async () => {
      renderWithQuery(<PrivacyPage />);

      expect(await screen.findByText("Nothing deleted")).toBeTruthy();
    });
  });

  it("states that Hamster uses no internet", () => {
    renderWithQuery(<PrivacyPage />);

    expect(screen.getByText("Internet access")).toBeTruthy();
    expect(screen.getByText("None")).toBeTruthy();
  });
});
