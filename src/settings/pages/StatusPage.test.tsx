import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../../test/apiMock")).createApiMock() }));

import { api } from "../../api";
import { useStore } from "../../state/store";
import { RevealTarget } from "../controls";
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
    expect(screen.getByText("Stored PDFs")).toBeTruthy();
    expect(screen.getByText("Checked against their checksums")).toBeTruthy();
    expect(screen.getByText("41 of 41")).toBeTruthy();
  });

  it("anchors a group by its id, not its shown title", async () => {
    const scroll = vi.fn();
    Element.prototype.scrollIntoView = scroll;
    mocked.statusView.mockResolvedValue([statusGroup({ id: "statements", title: "Kontoauszüge" })]);
    renderWithQuery(
      <RevealTarget.Provider value={{ id: "status-statements" }}>
        <StatusPage />
      </RevealTarget.Provider>,
    );

    const region = await screen.findByRole("region", { name: "Kontoauszüge" });

    await waitFor(() => expect(scroll.mock.contexts).toContain(region));
    delete (Element.prototype as { scrollIntoView?: unknown }).scrollIntoView;
  });

  it("says a group has nothing to check when it has no rows", async () => {
    mocked.statusView.mockResolvedValue([statusGroup({ title: "Rules", rows: [] })]);
    renderWithQuery(<StatusPage />);

    expect(await screen.findByText("Rules")).toBeTruthy();
    expect(screen.getByText("Nothing to check yet")).toBeTruthy();
  });

  describe("tone", () => {
    it.each([
      ["ok", "OK"],
      ["warning", "Warning"],
      ["error", "Error"],
    ] as const)("marks a %s row with the %s icon", async (tone, name) => {
      mocked.statusView.mockResolvedValue([statusGroup({ rows: [statusRow({ tone, value: "Some value" })] })]);
      renderWithQuery(<StatusPage />);

      const button = await screen.findByRole("button", { name: /Some value/ });

      expect(within(button).getByRole("img", { name })).toBeTruthy();
    });

    it("shows no icon for an info row", async () => {
      mocked.statusView.mockResolvedValue([statusGroup({ rows: [statusRow({ tone: "info", value: "41 PDFs" })] })]);
      renderWithQuery(<StatusPage />);

      expect(await screen.findByRole("button", { name: "41 PDFs" })).toBeTruthy();
      expect(screen.queryByRole("img")).toBeNull();
    });
  });

  describe("copying", () => {
    it("copies the value on click and confirms the row's label with a toast", async () => {
      mocked.statusView.mockResolvedValue([statusGroup({ rows: [statusRow({ label: "Statement chains", value: "1 gap", tone: "warning" })] })]);
      renderWithQuery(<StatusPage />);

      await userEvent.click(await screen.findByRole("button", { name: /1 gap/ }));

      expect(navigator.clipboard.writeText).toHaveBeenCalledWith("1 gap");
      await waitFor(() => expect(useStore.getState().toasts.map((t) => t.message)).toEqual(["Copied Statement chains"]));
    });

    it("copies the row's copy text instead of the shown value when it has one", async () => {
      mocked.statusView.mockResolvedValue([
        statusGroup({ rows: [statusRow({ label: "Data folder", value: "…/Hamster", tone: "info", copyText: "/Users/demo/Library/Hamster" })] }),
      ]);
      renderWithQuery(<StatusPage />);

      await userEvent.click(await screen.findByRole("button", { name: "…/Hamster" }));

      expect(navigator.clipboard.writeText).toHaveBeenCalledWith("/Users/demo/Library/Hamster");
      await waitFor(() => expect(useStore.getState().toasts.map((t) => t.message)).toEqual(["Copied Data folder"]));
    });

    it("shows no toast when the clipboard refuses the value", async () => {
      mocked.statusView.mockResolvedValue([statusGroup({ rows: [statusRow({ value: "Fine" })] })]);
      vi.mocked(navigator.clipboard.writeText).mockRejectedValueOnce(new Error("denied"));
      renderWithQuery(<StatusPage />);

      await userEvent.click(await screen.findByRole("button", { name: /Fine/ }));

      expect(navigator.clipboard.writeText).toHaveBeenCalledWith("Fine");
      expect(useStore.getState().toasts).toEqual([]);
    });
  });
});
