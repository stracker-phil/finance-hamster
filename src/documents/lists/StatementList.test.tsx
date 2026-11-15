import { screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../../test/apiMock")).createApiMock() }));

import { api } from "../../api";
import { HOME } from "../../state/route";
import { useStore } from "../../state/store";
import { accountStatements, statementListRow } from "../../test/documents";
import { renderWithQuery } from "../../test/renderWithQuery";
import { StatementList } from "./StatementList";

const mocked = vi.mocked(api, true);

beforeEach(() => {
  vi.clearAllMocks();
  useStore.setState({ route: HOME, back: [], forward: [] });
  mocked.statementsList.mockResolvedValue([]);
});

describe("StatementList", () => {
  it("groups the statements by account, each row with its number, period, line count and closing balance", async () => {
    mocked.statementsList.mockResolvedValue([
      accountStatements({ statements: [statementListRow(), statementListRow({ id: "st-9", title: "Nr. 009/2026", period: "1 Oct 2026 – 31 Oct 2026", closing: "€1,412.30", lines: "1 line" })] }),
      accountStatements({ accountId: "acc-2", name: "Savings", statements: [statementListRow({ id: "st-s1", title: "Nr. 001/2026" })] }),
    ]);

    renderWithQuery(<StatementList query="" empty={null} />);

    const group = await screen.findByRole("region", { name: "Private" });
    const first = within(within(group).getByRole("button", { name: /Nr\. 010\/2026/ }));
    expect(first.getByText("1 Nov 2026 – 10 Nov 2026 · 42 lines")).toBeTruthy();
    expect(first.getByText("€1,675.86")).toBeTruthy();
    expect(within(group).getByRole("button", { name: /Nr\. 009\/2026/ }).textContent).toContain("1 Oct 2026 – 31 Oct 2026 · 1 line");
    expect(within(screen.getByRole("region", { name: "Savings" })).getByRole("button", { name: /Nr\. 001\/2026/ })).toBeTruthy();
  });

  it("asks for the statements matching the query", async () => {
    mocked.statementsList.mockResolvedValue([accountStatements()]);

    renderWithQuery(<StatementList query="nov" empty={null} />);

    await screen.findByRole("region", { name: "Private" });
    expect(mocked.statementsList).toHaveBeenCalledWith("nov");
  });

  it("opens the statement when its row is clicked", async () => {
    mocked.statementsList.mockResolvedValue([accountStatements()]);
    renderWithQuery(<StatementList query="" empty={null} />);

    await userEvent.click(await screen.findByRole("button", { name: /Nr\. 010\/2026/ }));

    expect(useStore.getState().route).toEqual({ page: "statement", statementId: "st-10" });
  });

  it("opens the account when its name is clicked", async () => {
    mocked.statementsList.mockResolvedValue([accountStatements({ accountId: "acc-2", name: "Savings" })]);
    renderWithQuery(<StatementList query="" empty={null} />);

    await userEvent.click(await screen.findByRole("button", { name: "Savings" }));

    expect(useStore.getState().route).toEqual({ page: "account", accountId: "acc-2" });
  });

  it("shows what it is given instead when there are no statements", async () => {
    renderWithQuery(<StatementList query="" empty={<p>Nothing yet</p>} />);

    expect(await screen.findByText("Nothing yet")).toBeTruthy();
  });

  it("does not show the empty content while the statements are still loading", () => {
    mocked.statementsList.mockReturnValue(new Promise(() => {}));

    renderWithQuery(<StatementList query="" empty={<p>Nothing yet</p>} />);

    expect(screen.queryByText("Nothing yet")).toBeNull();
  });
});
