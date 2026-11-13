import { act, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { account } from "../test/accounts";
import { renderWithQuery } from "../test/renderWithQuery";
import { ruleView, rulesView } from "../test/lineRules";
import { reviewLine, reviewQueue } from "../test/review";
import { accountCoverage, accountLines, coverage, lineView, statementGroup } from "../test/statements";
import { useAccounts } from "./accounts";
import { useLineRules } from "./lineRules";
import { useReviewQueue } from "./review";
import { useAccountLines, useCoverage } from "./statements";
import { useTaxYears } from "./taxYears";
import { useEvents } from "./useEvents";

const mocked = vi.mocked(api, true);

function Probe() {
  useEvents();
  const { data: accounts } = useAccounts();
  const { data: years } = useTaxYears();
  return (
    <>
      <p>Accounts: {accounts?.map((a) => a.name).join(", ")}</p>
      <p>Years: {years?.map((y) => y.year).join(", ")}</p>
    </>
  );
}

async function emitDataChanged(tables: string[]) {
  await waitFor(() => expect(mocked.listen.mock.calls.some(([name]) => name === "data_changed")).toBe(true));
  const handler = mocked.listen.mock.calls.find(([name]) => name === "data_changed")![1] as (payload: { tables: string[] }) => void;
  await act(async () => handler({ tables }));
}

beforeEach(() => {
  vi.clearAllMocks();
  mocked.listen.mockResolvedValue(() => {});
  mocked.listAccounts.mockResolvedValue([account({ name: "Private" })]);
  mocked.listTaxYears.mockResolvedValue([{ year: 2026, state: "open", ruleset: "2026 v1" }]);
});

describe("useEvents", () => {
  it("refetches the accounts when data_changed names the accounts table", async () => {
    renderWithQuery(<Probe />);
    await screen.findByText("Accounts: Private");
    mocked.listAccounts.mockResolvedValue([account({ name: "Private" }), account({ id: "acc-2", name: "Savings" })]);

    await emitDataChanged(["accounts"]);

    expect(await screen.findByText("Accounts: Private, Savings")).toBeTruthy();
    expect(screen.getByText("Years: 2026")).toBeTruthy();
  });

  it("refetches the tax years when data_changed names the tax_years table", async () => {
    renderWithQuery(<Probe />);
    await screen.findByText("Years: 2026");
    mocked.listTaxYears.mockResolvedValue([
      { year: 2027, state: "open", ruleset: "2027 v1" },
      { year: 2026, state: "closed", ruleset: "2026 v1" },
    ]);

    await emitDataChanged(["tax_years"]);

    expect(await screen.findByText("Years: 2027, 2026")).toBeTruthy();
    expect(screen.getByText("Accounts: Private")).toBeTruthy();
  });

  describe("statement tables", () => {
    function StatementsProbe() {
      useEvents();
      const { data: coverage } = useCoverage();
      const { data: lines } = useAccountLines("acc-1");
      return (
        <>
          <p>Coverage: {coverage?.accounts.map((a) => a.last).join(", ")}</p>
          <p>Lines: {lines?.groups.map((g) => g.label).join(", ")}</p>
        </>
      );
    }

    const loaded = async () => {
      renderWithQuery(<StatementsProbe />);
      await screen.findByText("Coverage: Nr. 009 · until 31 Oct");
      await screen.findByText("Lines: Nr. 009/2026");
    };

    const newer = () => {
      mocked.statementsCoverage.mockResolvedValue(coverage({ accounts: [accountCoverage({ last: "Nr. 010 · until 10 Nov" })] }));
      mocked.accountLines.mockResolvedValue(accountLines({ groups: [statementGroup({ label: "Nr. 010/2026" }), statementGroup({ id: "st-0", label: "Nr. 009/2026" })] }));
    };

    beforeEach(() => {
      mocked.statementsCoverage.mockResolvedValue(coverage({ accounts: [accountCoverage({ last: "Nr. 009 · until 31 Oct" })] }));
      mocked.accountLines.mockResolvedValue(accountLines({ groups: [statementGroup({ label: "Nr. 009/2026" })] }));
    });

    it.each(["statements", "accounts"])("refetches the coverage and the account lines when data_changed names the %s table", async (table) => {
      await loaded();
      newer();

      await emitDataChanged([table]);

      expect(await screen.findByText("Coverage: Nr. 010 · until 10 Nov")).toBeTruthy();
      expect(await screen.findByText("Lines: Nr. 010/2026, Nr. 009/2026")).toBeTruthy();
    });

    it("refetches the coverage and the account lines when data_changed names the transactions table", async () => {
      await loaded();
      newer();

      await emitDataChanged(["transactions"]);

      expect(await screen.findByText("Lines: Nr. 010/2026, Nr. 009/2026")).toBeTruthy();
      expect(await screen.findByText("Coverage: Nr. 010 · until 10 Nov")).toBeTruthy();
    });
  });

  describe("review", () => {
    function ReviewProbe() {
      useEvents();
      const { data } = useReviewQueue();
      return <p>Review: {data?.lines.map((l) => l.title).join(", ")}</p>;
    }

    it.each(["decisions", "transactions", "accounts", "tax_years"])("refetches the review queue when data_changed names the %s table", async (table) => {
      mocked.reviewQueue.mockResolvedValue(reviewQueue());
      renderWithQuery(<ReviewProbe />);
      await screen.findByText("Review: Example Bookshop");
      mocked.reviewQueue.mockResolvedValue(reviewQueue({ lines: [reviewLine({ title: "Example Cinema" })] }));

      await emitDataChanged([table]);

      expect(await screen.findByText("Review: Example Cinema")).toBeTruthy();
    });

    it("keeps the review queue when data_changed names an unrelated table", async () => {
      mocked.reviewQueue.mockResolvedValue(reviewQueue());
      renderWithQuery(<ReviewProbe />);
      await screen.findByText("Review: Example Bookshop");
      mocked.reviewQueue.mockResolvedValue(reviewQueue({ lines: [] }));

      await emitDataChanged(["documents"]);

      expect(screen.getByText("Review: Example Bookshop")).toBeTruthy();
    });

    it("refetches the coverage and the account lines when data_changed names the decisions table", async () => {
      function DecisionsProbe() {
        useEvents();
        const { data: cov } = useCoverage();
        const { data: lines } = useAccountLines("acc-1");
        return (
          <>
            <p>Coverage: {cov?.accounts.map((a) => a.toReview).join(", ")}</p>
            <p>Lines: {lines?.groups.flatMap((g) => g.lines.map((l) => l.status)).join(", ")}</p>
          </>
        );
      }
      mocked.statementsCoverage.mockResolvedValue(coverage({ accounts: [accountCoverage({ toReview: 1 })] }));
      mocked.accountLines.mockResolvedValue(accountLines({ groups: [statementGroup({ lines: [lineView({ status: "review" })] })] }));
      renderWithQuery(<DecisionsProbe />);
      await screen.findByText("Coverage: 1");
      await screen.findByText("Lines: review");
      mocked.statementsCoverage.mockResolvedValue(coverage({ accounts: [accountCoverage({ toReview: 0 })] }));
      mocked.accountLines.mockResolvedValue(accountLines({ groups: [statementGroup({ lines: [lineView({ status: "anv" })] })] }));

      await emitDataChanged(["decisions"]);

      expect(await screen.findByText("Coverage: 0")).toBeTruthy();
      expect(await screen.findByText("Lines: anv")).toBeTruthy();
    });
  });

  describe("line rules", () => {
    function RulesProbe() {
      useEvents();
      const { data } = useLineRules();
      return <p>Rules: {data?.groups.flatMap((g) => g.rules.map((r) => r.label)).join(", ")}</p>;
    }

    it.each(["rules", "decisions", "transactions", "accounts", "tax_years"])("refetches the line rules when data_changed names the %s table", async (table) => {
      mocked.rulesView.mockResolvedValue(rulesView());
      renderWithQuery(<RulesProbe />);
      await screen.findByText("Rules: Bank text contains ALPENMARKT");
      mocked.rulesView.mockResolvedValue(rulesView({ groups: [{ title: "Ignore", rules: [ruleView({ label: "Bank text contains BILLA" })] }] }));

      await emitDataChanged([table]);

      expect(await screen.findByText("Rules: Bank text contains BILLA")).toBeTruthy();
    });

    it("keeps the line rules when data_changed names an unrelated table", async () => {
      mocked.rulesView.mockResolvedValue(rulesView());
      renderWithQuery(<RulesProbe />);
      await screen.findByText("Rules: Bank text contains ALPENMARKT");
      mocked.rulesView.mockResolvedValue(rulesView({ groups: [] }));

      await emitDataChanged(["documents"]);

      expect(screen.getByText("Rules: Bank text contains ALPENMARKT")).toBeTruthy();
    });
  });

  it("ignores tables it shows nothing of", async () => {
    renderWithQuery(<Probe />);
    await screen.findByText("Accounts: Private");
    mocked.listAccounts.mockResolvedValue([]);
    mocked.listTaxYears.mockResolvedValue([]);

    await emitDataChanged(["documents"]);

    expect(screen.getByText("Accounts: Private")).toBeTruthy();
    expect(screen.getByText("Years: 2026")).toBeTruthy();
  });
});
