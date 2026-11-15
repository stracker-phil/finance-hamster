import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { HOME } from "../state/route";
import { useStore } from "../state/store";
import { renderWithQuery } from "../test/renderWithQuery";
import { CloseYearSheet } from "./CloseYearSheet";

const mocked = vi.mocked(api, true);

beforeEach(() => {
  vi.clearAllMocks();
  useStore.setState({ route: HOME, back: [], forward: [], toasts: [] });
  mocked.yearCloseCheck.mockResolvedValue({ blockers: [], warnings: [] });
  mocked.yearClose.mockResolvedValue("Close Tax Year 2026");
});

describe("CloseYearSheet", () => {
  it("opens the close sheet directly", async () => {
    renderWithQuery(<CloseYearSheet year={2026} onClose={() => {}} />);

    expect(await screen.findByRole("dialog", { name: "Close Tax Year 2026?" })).toBeTruthy();
  });

  it("closes the year and the sheet when nothing blocks", async () => {
    const onClose = vi.fn();
    renderWithQuery(<CloseYearSheet year={2026} onClose={onClose} />);
    await screen.findByText("Everything is in place");

    await userEvent.click(screen.getByRole("button", { name: "Close Year" }));

    await vi.waitFor(() => expect(onClose).toHaveBeenCalled());
    expect(mocked.yearClose).toHaveBeenCalledWith(2026);
  });

  it("lists a blocker and keeps closing disabled", async () => {
    mocked.yearCloseCheck.mockResolvedValue({ blockers: ["1 draft is still open"], warnings: [] });
    renderWithQuery(<CloseYearSheet year={2026} onClose={() => {}} />);

    expect(await screen.findByText("1 draft is still open")).toBeTruthy();
    expect(screen.getByRole<HTMLButtonElement>("button", { name: "Close Year" }).disabled).toBe(true);
  });

  it("cancels without navigating", async () => {
    const onClose = vi.fn();
    renderWithQuery(<CloseYearSheet year={2026} onClose={onClose} />);

    await userEvent.click(await screen.findByRole("button", { name: "Cancel" }));

    expect(onClose).toHaveBeenCalled();
    expect(useStore.getState().route).toEqual(HOME);
  });
});
