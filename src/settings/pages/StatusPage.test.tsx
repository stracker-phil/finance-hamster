import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../../test/apiMock")).createApiMock() }));

import { api } from "../../api";
import { useStore } from "../../state/store";
import { statusGroup, statusRow } from "../../test/integrity";
import { renderWithQuery } from "../../test/renderWithQuery";
import { StatusPage } from "./StatusPage";

const mocked = vi.mocked(api, true);

describe("StatusPage", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useStore.setState({ toasts: [] });
    mocked.statusView.mockResolvedValue([]);
  });

  it("shows one group per title with each row's label, detail and value", async () => {
    mocked.statusView.mockResolvedValue([
      statusGroup(),
      statusGroup({ title: "Files", rows: [statusRow({ label: "Stored PDFs", detail: "Checked against their checksums", value: "41 of 41" })] }),
    ]);
    renderWithQuery(<StatusPage />);

    expect(await screen.findByText("Statements")).toBeTruthy();
    expect(screen.getByText("Files")).toBeTruthy();
    expect(screen.getByText("Private")).toBeTruthy();
    expect(screen.getByText("Balances chain from statement to statement")).toBeTruthy();
    expect(screen.getByRole("button", { name: "OK" })).toBeTruthy();
    expect(screen.getByText("Stored PDFs")).toBeTruthy();
    expect(screen.getByRole("button", { name: "41 of 41" })).toBeTruthy();
  });

  it("says a group has nothing to check when it has no rows", async () => {
    mocked.statusView.mockResolvedValue([statusGroup({ title: "Rules", rows: [] })]);
    renderWithQuery(<StatusPage />);

    expect(await screen.findByText("Rules")).toBeTruthy();
    expect(screen.getByText("Nothing to check yet")).toBeTruthy();
  });

  it("copies a value on click and confirms it with a toast", async () => {
    mocked.statusView.mockResolvedValue([statusGroup({ rows: [statusRow({ value: "1 gap", ok: false })] })]);
    renderWithQuery(<StatusPage />);

    await userEvent.click(await screen.findByRole("button", { name: "1 gap" }));

    expect(navigator.clipboard.writeText).toHaveBeenCalledWith("1 gap");
    await waitFor(() => expect(useStore.getState().toasts.map((t) => t.message)).toEqual(["Copied 1 gap"]));
  });

  it("shows no toast when the clipboard refuses the value", async () => {
    mocked.statusView.mockResolvedValue([statusGroup()]);
    vi.mocked(navigator.clipboard.writeText).mockRejectedValueOnce(new Error("denied"));
    renderWithQuery(<StatusPage />);

    await userEvent.click(await screen.findByRole("button", { name: "OK" }));

    expect(useStore.getState().toasts).toEqual([]);
  });
});
