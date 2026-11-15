import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../../test/apiMock")).createApiMock() }));

import { api } from "../../api";
import { useStore } from "../../state/store";
import { renderWithQuery } from "../../test/renderWithQuery";
import { AboutPage } from "./AboutPage";
import { SETTINGS_PAGES } from "./index";

const mocked = vi.mocked(api, true);

describe("AboutPage", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useStore.setState({ toasts: [] });
    mocked.appInfo.mockResolvedValue({ name: "Hamster", version: "2.4.1" });
    mocked.runCommand.mockResolvedValue(undefined);
  });

  it("shows the app version", async () => {
    renderWithQuery(<AboutPage />);

    expect(await screen.findByText("2.4.1")).toBeTruthy();
    expect(screen.getByText("Version")).toBeTruthy();
  });

  it("titles the page with the app's full name", async () => {
    renderWithQuery(<AboutPage />);
    await screen.findByText("2.4.1");

    expect(screen.getByRole("heading", { name: "Finance Hamster" })).toBeTruthy();
  });

  it("has a row for every About entry of the Settings search index", async () => {
    const rows = SETTINGS_PAGES.find((p) => p.id === "about")?.rows ?? [];
    renderWithQuery(<AboutPage />);
    await screen.findByText("2.4.1");

    expect(rows.map((r) => r.id)).toEqual(expect.arrayContaining(["about-data-folder", "about-log-folder"]));
    for (const row of rows.filter((r) => r.id.startsWith("about-"))) expect(screen.getByText(row.title)).toBeTruthy();
  });

  it.each([
    ["Data folder", "Show in Finder", "app:data"],
    ["Log folder", "Open", "app:logs"],
  ])("opens the %s through its button", async (_row, button, command) => {
    renderWithQuery(<AboutPage />);

    await userEvent.click(await screen.findByRole("button", { name: button }));

    expect(mocked.runCommand).toHaveBeenCalledWith(command);
    expect(useStore.getState().toasts).toEqual([]);
  });

  it.each([
    ["Show in Finder", "Data folder is missing"],
    ["Open", "Log folder is missing"],
  ])("shows an error toast when %s is rejected", async (button, message) => {
    mocked.runCommand.mockRejectedValue({ kind: "io", message, code: null });
    renderWithQuery(<AboutPage />);

    await userEvent.click(await screen.findByRole("button", { name: button }));

    await waitFor(() => expect(useStore.getState().toasts.map((t) => [t.kind, t.message])).toEqual([["error", message]]));
  });
});
