import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { HOME } from "../state/route";
import { useStore } from "../state/store";
import { renderWithQuery as render } from "../test/renderWithQuery";
import { accountCoverage, coverage, importPreview } from "../test/statements";
import { StatementsPage } from "./StatementsPage";

const mocked = vi.mocked(api, true);

beforeEach(() => {
  vi.clearAllMocks();
  useStore.setState({ route: HOME, back: [], forward: [], filter: "", toasts: [], importPreview: null });
  mocked.statementsCoverage.mockResolvedValue(coverage());
});

describe("StatementsPage", () => {
  it("shows each account with its last statement and state", async () => {
    render(<StatementsPage />);

    expect(await screen.findByText("Private")).toBeTruthy();
    expect(screen.getByText("Nr. 010 · until 10 Nov")).toBeTruthy();
    expect(screen.getByText("Up to date")).toBeTruthy();
  });

  it("asks for the accounts matching the header filter", async () => {
    useStore.setState({ filter: "sav" });

    render(<StatementsPage />);

    await screen.findByText("Private");
    expect(mocked.statementsCoverage).toHaveBeenCalledWith("sav");
  });

  it("shows the month heads and each month's tip", async () => {
    render(<StatementsPage />);

    await screen.findByText("Private");
    for (const label of ["Sep", "Oct", "Nov"]) expect(screen.getByText(label)).toBeTruthy();
    for (const tip of ["Sep: complete", "Oct: until 20 Oct", "Nov: no statement"]) expect(screen.getByTitle(tip)).toBeTruthy();
  });

  it("shows how far behind an account is and the note about a gap in its chain", async () => {
    mocked.statementsCoverage.mockResolvedValue(
      coverage({ accounts: [accountCoverage({ name: "Savings", state: "behind", stateLabel: "13 days behind", note: "Nr. 008 is missing between Nr. 007 and Nr. 009" })] }),
    );

    render(<StatementsPage />);

    expect(await screen.findByText("13 days behind")).toBeTruthy();
    expect(screen.getByText("Nr. 008 is missing between Nr. 007 and Nr. 009")).toBeTruthy();
  });

  it("shows no note for an account without a gap", async () => {
    render(<StatementsPage />);

    await screen.findByText("Private");
    expect(screen.queryByText(/is missing/)).toBeNull();
  });

  it("lists several accounts in order", async () => {
    mocked.statementsCoverage.mockResolvedValue(coverage({ accounts: [accountCoverage(), accountCoverage({ id: "acc-2", name: "Savings", last: "Nr. 004 · until 30 Sep" })] }));

    render(<StatementsPage />);

    await screen.findByText("Savings");
    const opens = screen.getAllByRole("button", { name: /^Open / }).map((b) => b.getAttribute("aria-label"));
    expect(opens).toEqual(["Open Private", "Open Savings"]);
  });

  it("opens an account's lines when its row is clicked", async () => {
    render(<StatementsPage />);

    await userEvent.click(await screen.findByRole("button", { name: "Open Private" }));

    expect(useStore.getState().route).toEqual({ page: "account", accountId: "acc-1" });
  });

  it("asks for a statement when Import… is clicked", async () => {
    const preview = importPreview();
    mocked.statementImportChoose.mockResolvedValue(preview);
    render(<StatementsPage />);

    await userEvent.click(await screen.findByRole("button", { name: "Import…" }));

    await waitFor(() => expect(useStore.getState().importPreview).toEqual(preview));
  });

  it("explains the legend of the month cells", async () => {
    render(<StatementsPage />);

    for (const label of ["Imported", "Partly", "Missing"]) expect(await screen.findByText(label)).toBeTruthy();
  });

  describe("review", () => {
    it("shows the state chip of an account whose lines wait for review", async () => {
      mocked.statementsCoverage.mockResolvedValue(coverage({ accounts: [accountCoverage({ state: "review", stateLabel: "3 to review", toReview: 3 })] }));

      render(<StatementsPage />);

      expect(await screen.findByText("3 to review")).toBeTruthy();
    });

    it("offers Start Review with the lines of all accounts summed", async () => {
      mocked.statementsCoverage.mockResolvedValue(
        coverage({ accounts: [accountCoverage({ toReview: 3 }), accountCoverage({ id: "acc-2", name: "Savings", toReview: 2 })] }),
      );

      render(<StatementsPage />);

      expect(await screen.findByText("5 lines wait for your decision.")).toBeTruthy();
    });

    it("says a single line waits in the singular", async () => {
      mocked.statementsCoverage.mockResolvedValue(coverage({ accounts: [accountCoverage({ toReview: 1 })] }));

      render(<StatementsPage />);

      expect(await screen.findByText("1 line waits for your decision.")).toBeTruthy();
    });

    it("opens the review when Start Review is clicked", async () => {
      mocked.statementsCoverage.mockResolvedValue(coverage({ accounts: [accountCoverage({ toReview: 3 })] }));
      render(<StatementsPage />);

      await userEvent.click(await screen.findByRole("button", { name: "Start Review" }));

      expect(useStore.getState().route).toEqual({ page: "review" });
    });

    it("shows no banner when no line waits", async () => {
      render(<StatementsPage />);

      await screen.findByText("Private");
      expect(screen.queryByRole("button", { name: "Start Review" })).toBeNull();
      expect(screen.queryByText(/for your decision/)).toBeNull();
    });
  });

  it("points to Settings when there is no account", async () => {
    mocked.statementsCoverage.mockResolvedValue(coverage({ accounts: [] }));

    render(<StatementsPage />);

    expect(await screen.findByText("No accounts")).toBeTruthy();
    expect(screen.getByText("Add an account in Settings → Accounts.")).toBeTruthy();
  });
});
