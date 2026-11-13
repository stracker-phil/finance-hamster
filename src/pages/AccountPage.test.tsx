import { screen, within } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { useStore } from "../state/store";
import { renderWithQuery as render } from "../test/renderWithQuery";
import { accountLines, lineView, statementGroup } from "../test/statements";
import { AccountPage } from "./AccountPage";

const mocked = vi.mocked(api, true);

beforeEach(() => {
  vi.clearAllMocks();
  useStore.setState({ toasts: [] });
  mocked.accountLines.mockResolvedValue(accountLines());
});

describe("AccountPage", () => {
  it("asks for the lines of the account it shows", async () => {
    render(<AccountPage accountId="acc-7" />);

    await screen.findByRole("region");
    expect(mocked.accountLines).toHaveBeenCalledWith("acc-7");
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

  it("says the account is gone and shows the error when it cannot be read", async () => {
    mocked.accountLines.mockRejectedValue({ kind: "not_found", message: "No such account", code: null });

    render(<AccountPage accountId="gone" />);

    expect(await screen.findByText("No such account")).toBeTruthy();
    expect(screen.getByText("It may have been removed.")).toBeTruthy();
  });
});
