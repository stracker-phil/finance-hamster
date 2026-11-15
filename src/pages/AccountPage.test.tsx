import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { HOME } from "../state/route";
import { useStore } from "../state/store";
import { withMenus } from "../test/popup";
import { renderWithQuery as render } from "../test/renderWithQuery";
import { reviewQueue } from "../test/review";
import { accountLines, lineView, statementGroup } from "../test/statements";
import { AccountPage } from "./AccountPage";

const mocked = vi.mocked(api, true);

beforeEach(() => {
  vi.clearAllMocks();
  useStore.setState({ route: HOME, back: [], forward: [], toasts: [], filter: "" });
  mocked.accountLines.mockResolvedValue(accountLines());
  mocked.reviewQueue.mockResolvedValue(reviewQueue({ lines: [] }));
});

describe("AccountPage", () => {
  it("asks for the lines of the account it shows", async () => {
    render(<AccountPage accountId="acc-7" />);

    await screen.findByRole("region");
    expect(mocked.accountLines).toHaveBeenCalledWith("acc-7", "");
  });

  it("asks for the lines matching the header filter", async () => {
    useStore.setState({ filter: "software" });
    render(<AccountPage accountId="acc-7" />);

    await screen.findByRole("region");
    expect(mocked.accountLines).toHaveBeenCalledWith("acc-7", "software");
  });

  it("shows each statement with its label, range and line count", async () => {
    mocked.accountLines.mockResolvedValue(
      accountLines({
        groups: [
          statementGroup({ id: "st-2", label: "Nr. 010/2026", range: "1 Nov – 10 Nov", count: "1 line" }),
          statementGroup({ id: "st-1", label: "Nr. 009/2026", range: "1 Oct – 31 Oct", count: "62 lines", lines: [lineView({ id: "line-9", title: "Example Cinema" })] }),
        ],
      }),
    );

    render(<AccountPage accountId="acc-1" />);

    const newest = await screen.findByRole("region", { name: "Nr. 010/2026" });
    expect(within(newest).getByText("1 Nov – 10 Nov")).toBeTruthy();
    expect(within(newest).getByText("1 line")).toBeTruthy();
    expect(within(newest).getByText("Example Market")).toBeTruthy();
    const older = screen.getByRole("region", { name: "Nr. 009/2026" });
    expect(within(older).getByText("62 lines")).toBeTruthy();
    expect(within(older).getByText("Example Cinema")).toBeTruthy();
    expect(screen.getAllByRole("region").map((r) => r.getAttribute("aria-label"))).toEqual(["Nr. 010/2026", "Nr. 009/2026"]);
  });

  it("notes a gap before a statement after its lines", async () => {
    mocked.accountLines.mockResolvedValue(accountLines({ groups: [statementGroup({ missingBefore: "No statement for 11–31 Oct" })] }));

    render(<AccountPage accountId="acc-1" />);

    const group = await screen.findByRole("region", { name: "Nr. 010/2026" });
    expect(within(group).getByRole("note").textContent).toBe("No statement for 11–31 Oct");
  });

  it("shows no note when nothing is missing before a statement", async () => {
    render(<AccountPage accountId="acc-1" />);

    await screen.findByRole("region", { name: "Nr. 010/2026" });
    expect(screen.queryByRole("note")).toBeNull();
  });

  it("shows a line's date, title and amount", async () => {
    render(<AccountPage accountId="acc-1" />);

    const group = await screen.findByRole("region", { name: "Nr. 010/2026" });
    expect(within(group).getByText("10 Nov")).toBeTruthy();
    expect(within(group).getByText("Example Market")).toBeTruthy();
    expect(within(group).getByText("−€22.48")).toBeTruthy();
    expect(within(group).queryByText("Transfer")).toBeNull();
  });

  it("shows the original amount of a card payment in a foreign currency", async () => {
    mocked.accountLines.mockResolvedValue(accountLines({ groups: [statementGroup({ lines: [lineView({ title: "Example Software", original: "USD 24.00", amount: "−€22.10" })] })] }));

    render(<AccountPage accountId="acc-1" />);

    expect(await screen.findByText("USD 24.00")).toBeTruthy();
    expect(screen.getByText("−€22.10")).toBeTruthy();
  });

  it("marks a transfer between own accounts", async () => {
    mocked.accountLines.mockResolvedValue(accountLines({ groups: [statementGroup({ lines: [lineView({ title: "To Savings", transfer: true })] })] }));

    render(<AccountPage accountId="acc-1" />);

    expect(await screen.findByText("Transfer")).toBeTruthy();
  });

  it.each([
    ["anv", "ANV", "KZ 720"],
    ["subscription", "Subscription", "Matched by rule"],
  ] as const)("marks a %s line with its chip and note", async (status, chip, note) => {
    mocked.accountLines.mockResolvedValue(accountLines({ groups: [statementGroup({ lines: [lineView({ status, note })] })] }));

    render(<AccountPage accountId="acc-1" />);

    expect(await screen.findByText(chip)).toBeTruthy();
    expect(screen.getByText(note)).toBeTruthy();
  });

  it.each([
    ["ignored", "Ignored by rule"],
    ["review", "Waiting for review"],
  ] as const)("shows the note of a %s line without a chip", async (status, note) => {
    mocked.accountLines.mockResolvedValue(accountLines({ groups: [statementGroup({ lines: [lineView({ status, note })] })] }));

    render(<AccountPage accountId="acc-1" />);

    expect(await screen.findByText(note)).toBeTruthy();
    expect(screen.queryByText("ANV")).toBeNull();
    expect(screen.queryByText("Subscription")).toBeNull();
  });

  it("shows no note for a line without one", async () => {
    render(<AccountPage accountId="acc-1" />);

    const group = await screen.findByRole("region", { name: "Nr. 010/2026" });
    expect(within(group).queryByText(/KZ|rule|review/)).toBeNull();
  });

  it("shows a credit with its plus sign", async () => {
    mocked.accountLines.mockResolvedValue(accountLines({ groups: [statementGroup({ lines: [lineView({ title: "Example Employer", amount: "+€2,850.00", credit: true })] })] }));

    render(<AccountPage accountId="acc-1" />);

    expect(await screen.findByText("+€2,850.00")).toBeTruthy();
  });

  it("says so when the account has no imported statement", async () => {
    mocked.accountLines.mockResolvedValue(accountLines({ groups: [] }));

    render(<AccountPage accountId="acc-1" />);

    expect(await screen.findByText("No lines")).toBeTruthy();
    expect(screen.getByText("This account has no imported statement yet.")).toBeTruthy();
  });

  describe("changing a line", () => {
    const withLine = (patch: Parameters<typeof lineView>[0]) => mocked.accountLines.mockResolvedValue(accountLines({ groups: [statementGroup({ lines: [lineView({ title: "Example Market", ...patch })] })] }));

    it("names each line row after its title", async () => {
      render(<AccountPage accountId="acc-1" />);

      expect(await screen.findByRole("button", { name: "Change Example Market" })).toBeTruthy();
    });

    it.each(["anv", "subscription", "ignored"] as const)("opens the sheet for a %s line", async (status) => {
      withLine({ status, itemKey: "kz-169" });
      render(withMenus(<AccountPage accountId="acc-1" />));

      await userEvent.click(await screen.findByRole("button", { name: "Change Example Market" }));

      const dialog = await screen.findByRole("dialog", { name: "What is this line?" });
      expect(within(dialog).getByText("Example Market · −€22.48")).toBeTruthy();
    });

    it("opens the review for a line waiting for a decision, without a sheet", async () => {
      withLine({ status: "review", note: "Waiting for review" });
      render(<AccountPage accountId="acc-1" />);

      await userEvent.click(await screen.findByRole("button", { name: "Change Example Market" }));

      expect(useStore.getState().route).toEqual({ page: "review" });
      expect(screen.queryByRole("dialog")).toBeNull();
    });

    it("disables the row of a transfer", async () => {
      withLine({ status: "transfer", transfer: true });
      render(<AccountPage accountId="acc-1" />);

      const row = await screen.findByRole<HTMLButtonElement>("button", { name: "Change Example Market" });
      expect(row.disabled).toBe(true);
    });

    it("closes the sheet with Cancel", async () => {
      withLine({ status: "ignored" });
      render(withMenus(<AccountPage accountId="acc-1" />));
      await userEvent.click(await screen.findByRole("button", { name: "Change Example Market" }));

      await userEvent.click(await screen.findByRole("button", { name: "Cancel" }));

      expect(screen.queryByRole("dialog")).toBeNull();
      expect(mocked.reviewDecide).not.toHaveBeenCalled();
    });

    it("closes the sheet after a saved change", async () => {
      withLine({ status: "anv", itemKey: "kz-169" });
      mocked.reviewDecide.mockResolvedValue("Ignore: Example Market");
      render(withMenus(<AccountPage accountId="acc-1" />));
      await userEvent.click(await screen.findByRole("button", { name: "Change Example Market" }));
      await userEvent.click(await screen.findByRole("button", { name: "Ignore" }));

      await userEvent.click(screen.getByRole("button", { name: "Save" }));

      await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    });
  });

  it("says the account is gone and shows the error when it cannot be read", async () => {
    mocked.accountLines.mockRejectedValue({ kind: "not_found", message: "No such account", code: null });

    render(<AccountPage accountId="gone" />);

    expect(await screen.findByText("No such account")).toBeTruthy();
    expect(screen.getByText("It may have been removed.")).toBeTruthy();
  });
});
