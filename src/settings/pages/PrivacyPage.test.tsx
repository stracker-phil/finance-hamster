import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../../test/apiMock")).createApiMock() }));

import { api } from "../../api";
import { useStore } from "../../state/store";
import { renderWithQuery } from "../../test/renderWithQuery";
import { PrivacyPage } from "./PrivacyPage";

const mocked = vi.mocked(api, true);

describe("PrivacyPage", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useStore.setState({ toasts: [] });
    mocked.dataFolder.mockResolvedValue("/Users/demo/Library/Application Support/com.example.hamster/");
    mocked.showDataFolder.mockResolvedValue(undefined);
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

  it("states that Hamster uses no internet", () => {
    renderWithQuery(<PrivacyPage />);

    expect(screen.getByText("Internet access")).toBeTruthy();
    expect(screen.getByText("None")).toBeTruthy();
  });
});
