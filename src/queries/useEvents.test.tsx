import { act, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { account } from "../test/accounts";
import { renderWithQuery } from "../test/renderWithQuery";
import { accountStatements, archivedDocument, deletionView, documentGroup, documentRow, documentView, historyEntry, lineChoice, otherGroup, otherListRow, otherView, statementListRow, statementView, vendorView, vendorsView } from "../test/documents";
import { fileCheck, fileProblem, statusGroup } from "../test/integrity";
import { payslipView, payslipsYear } from "../test/payslips";
import { ruleView, rulesView } from "../test/lineRules";
import { reviewLine, reviewQueue } from "../test/review";
import { accountCoverage, accountLines, coverage, lineView, statementGroup } from "../test/statements";
import { subscriptionDetail, subscriptionForm, subscriptionsView } from "../test/subscriptions";
import { step, todayView } from "../test/today";
import { categoryChoice, elsewhere, yearPage } from "../test/year";
import { useAccounts } from "./accounts";
import { useCommands } from "./commands";
import { useArchive, useDeletions, useDocument, useDocumentHistory, useDocumentLineOptions, useDocuments, useVendors, useYearLog } from "./documents";
import { useLastFileCheck, useStatus } from "./integrity";
import { useLineRules } from "./lineRules";
import { usePayslip, usePayslipsYear } from "./payslips";
import { useReviewQueue } from "./review";
import { useOther, useOthers } from "./others";
import { useAccountLines, useCoverage, useStatement, useStatementsList } from "./statements";
import { useMatchCount, useSubscription, useSubscriptions } from "./subscriptions";
import { useTaxYears } from "./taxYears";
import { useToday } from "./today";
import { useElsewhere, useYearCategories, useYearPage } from "./year";
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

  describe("statements and other documents as documents", () => {
    function DocumentKindsProbe() {
      useEvents();
      const { data: list } = useStatementsList("");
      const { data: statement } = useStatement("st-10");
      const { data: groups } = useOthers("");
      const { data: other } = useOther("other-1");
      return (
        <>
          <p>Statements: {list?.flatMap((a) => a.statements.map((s) => s.title)).join(", ")}</p>
          <p>Statement: {statement?.title}</p>
          <p>Others: {groups?.flatMap((g) => g.documents.map((d) => d.title)).join(", ")}</p>
          <p>Other: {other?.title}</p>
        </>
      );
    }

    const shown = {
      statements: ["Statements: Nr. 009/2026", "Statements: Nr. 010/2026"],
      statement: ["Statement: Private · Nr. 009/2026", "Statement: Private · Nr. 010/2026"],
      others: ["Others: Old Lease", "Others: New Lease"],
      other: ["Other: Old Lease", "Other: New Lease"],
    } as const;

    beforeEach(() => {
      mocked.statementsList.mockResolvedValue([accountStatements({ statements: [statementListRow({ title: "Nr. 009/2026" })] })]);
      mocked.statementView.mockResolvedValue(statementView({ title: "Private · Nr. 009/2026" }));
      mocked.othersList.mockResolvedValue([otherGroup({ documents: [otherListRow({ title: "Old Lease" })] })]);
      mocked.otherView.mockResolvedValue(otherView({ title: "Old Lease" }));
    });

    it.each([
      ["statements", ["statements", "statement"]],
      ["accounts", ["statements", "statement"]],
      ["transactions", ["statements", "statement"]],
      ["others", ["others", "other"]],
      ["documents", []],
    ] as const)("when data_changed names the %s table, refetches %j and keeps the rest", async (table, refreshed) => {
      renderWithQuery(<DocumentKindsProbe />);
      for (const [old] of Object.values(shown)) await screen.findByText(old);
      mocked.statementsList.mockResolvedValue([accountStatements({ statements: [statementListRow({ title: "Nr. 010/2026" })] })]);
      mocked.statementView.mockResolvedValue(statementView({ title: "Private · Nr. 010/2026" }));
      mocked.othersList.mockResolvedValue([otherGroup({ documents: [otherListRow({ title: "New Lease" })] })]);
      mocked.otherView.mockResolvedValue(otherView({ title: "New Lease" }));

      await emitDataChanged([table]);

      for (const key of refreshed) expect(await screen.findByText(shown[key][1])).toBeTruthy();
      for (const key of Object.keys(shown) as (keyof typeof shown)[]) {
        if (!(refreshed as readonly string[]).includes(key)) expect(screen.getByText(shown[key][0])).toBeTruthy();
      }
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

  describe("documents", () => {
    function DocumentsProbe() {
      useEvents();
      const { data: doc } = useDocument("doc-1");
      const { data: groups } = useDocuments("");
      const { data: vendors } = useVendors();
      const { data: options } = useDocumentLineOptions("doc-1", "", true);
      return (
        <>
          <p>Document: {doc?.title}</p>
          <p>List: {groups?.flatMap((g) => g.documents.map((d) => d.title)).join(", ")}</p>
          <p>Vendors: {vendors?.vendors.map((v) => v.name).join(", ")}</p>
          <p>Options: {options?.map((o) => o.title).join(", ")}</p>
        </>
      );
    }

    const shown = {
      document: ["Document: Old Invoice", "Document: New Invoice"],
      list: ["List: Old Invoice", "List: New Invoice"],
      vendors: ["Vendors: Old Vendor", "Vendors: New Vendor"],
      options: ["Options: Old Line", "Options: New Line"],
    } as const;

    beforeEach(() => {
      mocked.documentView.mockResolvedValue(documentView({ title: "Old Invoice" }));
      mocked.documentsList.mockResolvedValue([documentGroup({ documents: [documentRow({ title: "Old Invoice" })] })]);
      mocked.vendorsView.mockResolvedValue(vendorsView({ vendors: [vendorView({ name: "Old Vendor" })] }));
      mocked.documentLineOptions.mockResolvedValue([lineChoice({ title: "Old Line" })]);
    });

    it.each([
      ["documents", ["document", "list", "vendors", "options"]],
      ["vendors", ["document", "list", "vendors"]],
      ["document_links", ["document", "list", "options"]],
      ["tax_years", ["document", "list", "vendors"]],
      ["transactions", ["options"]],
    ] as const)("when data_changed names the %s table, refetches %j and keeps the rest", async (table, refreshed) => {
      renderWithQuery(<DocumentsProbe />);
      for (const [old] of Object.values(shown)) await screen.findByText(old);
      mocked.documentView.mockResolvedValue(documentView({ title: "New Invoice" }));
      mocked.documentsList.mockResolvedValue([documentGroup({ documents: [documentRow({ title: "New Invoice" })] })]);
      mocked.vendorsView.mockResolvedValue(vendorsView({ vendors: [vendorView({ name: "New Vendor" })] }));
      mocked.documentLineOptions.mockResolvedValue([lineChoice({ title: "New Line" })]);

      await emitDataChanged([table]);

      for (const key of refreshed) expect(await screen.findByText(shown[key][1])).toBeTruthy();
      for (const key of Object.keys(shown) as (keyof typeof shown)[]) {
        if (!(refreshed as readonly string[]).includes(key)) expect(screen.getByText(shown[key][0])).toBeTruthy();
      }
    });
  });

  describe("history, log and archive", () => {
    function LifecycleProbe() {
      useEvents();
      const { data: history } = useDocumentHistory("doc-1");
      const { data: log } = useYearLog(2026);
      const { data: archive } = useArchive(2026);
      const { data: deletions } = useDeletions();
      return (
        <>
          <p>History: {history?.map((h) => h.text).join(", ")}</p>
          <p>Log: {log?.map((h) => h.text).join(", ")}</p>
          <p>Archive: {archive?.map((a) => a.title).join(", ")}</p>
          <p>Deletions: {deletions?.map((d) => d.title).join(", ")}</p>
        </>
      );
    }

    const shown = {
      history: ["History: Old entry", "History: New entry"],
      log: ["Log: Old entry", "Log: New entry"],
      archive: ["Archive: Old Archived", "Archive: New Archived"],
      deletions: ["Deletions: Old Deleted", "Deletions: New Deleted"],
    } as const;

    beforeEach(() => {
      mocked.documentHistory.mockResolvedValue([historyEntry({ text: "Old entry" })]);
      mocked.yearLog.mockResolvedValue([historyEntry({ text: "Old entry" })]);
      mocked.archiveList.mockResolvedValue([archivedDocument({ title: "Old Archived" })]);
      mocked.deletionLog.mockResolvedValue([deletionView({ title: "Old Deleted" })]);
    });

    it.each([
      ["documents", ["history", "log", "archive"]],
      ["vendors", ["history", "log"]],
      ["document_links", ["history", "log"]],
      ["deletions", ["deletions", "log", "archive"]],
      ["accounts", []],
    ] as const)("when data_changed names the %s table, refetches %j and keeps the rest", async (table, refreshed) => {
      renderWithQuery(<LifecycleProbe />);
      for (const [old] of Object.values(shown)) await screen.findByText(old);
      mocked.documentHistory.mockResolvedValue([historyEntry({ text: "New entry" })]);
      mocked.yearLog.mockResolvedValue([historyEntry({ text: "New entry" })]);
      mocked.archiveList.mockResolvedValue([archivedDocument({ title: "New Archived" })]);
      mocked.deletionLog.mockResolvedValue([deletionView({ title: "New Deleted" })]);

      await emitDataChanged([table]);

      for (const key of refreshed) expect(await screen.findByText(shown[key][1])).toBeTruthy();
      for (const key of Object.keys(shown) as (keyof typeof shown)[]) {
        if (!(refreshed as readonly string[]).includes(key)) expect(screen.getByText(shown[key][0])).toBeTruthy();
      }
    });
  });

  describe("year page and matches elsewhere", () => {
    function YearProbe() {
      useEvents();
      const { data: page } = useYearPage(2026, "");
      const { data: matches } = useElsewhere("pixel");
      return (
        <>
          <p>Year: {page?.total}</p>
          <p>Elsewhere: {matches?.map((m) => m.label).join(", ")}</p>
        </>
      );
    }

    const shown = {
      year: ["Year: €100.00", "Year: €200.00"],
      elsewhere: ["Elsewhere: Old match", "Elsewhere: New match"],
    } as const;

    beforeEach(() => {
      mocked.yearPage.mockResolvedValue(yearPage({ total: "€100.00" }));
      mocked.searchElsewhere.mockResolvedValue([elsewhere({ label: "Old match" })]);
    });

    it.each([
      ["answers", ["year"]],
      ["entries", ["year"]],
      ["documents", ["year", "elsewhere"]],
      ["vendors", ["year", "elsewhere"]],
      ["document_links", ["year", "elsewhere"]],
      ["decisions", ["year", "elsewhere"]],
      ["transactions", ["year", "elsewhere"]],
      ["deletions", ["year", "elsewhere"]],
      ["tax_years", ["year", "elsewhere"]],
      ["accounts", []],
    ] as const)("when data_changed names the %s table, refetches %j and keeps the rest", async (table, refreshed) => {
      renderWithQuery(<YearProbe />);
      for (const [old] of Object.values(shown)) await screen.findByText(old);
      mocked.yearPage.mockResolvedValue(yearPage({ total: "€200.00" }));
      mocked.searchElsewhere.mockResolvedValue([elsewhere({ label: "New match" })]);

      await emitDataChanged([table]);

      for (const key of refreshed) expect(await screen.findByText(shown[key][1])).toBeTruthy();
      for (const key of Object.keys(shown) as (keyof typeof shown)[]) {
        if (!(refreshed as readonly string[]).includes(key)) expect(screen.getByText(shown[key][0])).toBeTruthy();
      }
    });
  });

  describe("payslips", () => {
    function PayslipsProbe() {
      useEvents();
      const { data: panel } = usePayslipsYear(2026);
      const { data: payslip } = usePayslip("ps-7");
      const { data: page } = useYearPage(2026, "");
      return (
        <>
          <p>Panel: {panel?.telework}</p>
          <p>Payslip: {payslip?.title}</p>
          <p>Year: {page?.total}</p>
        </>
      );
    }

    const shown = {
      panel: ["Panel: Old days", "Panel: New days"],
      payslip: ["Payslip: Old title", "Payslip: New title"],
      year: ["Year: €100.00", "Year: €200.00"],
    } as const;

    beforeEach(() => {
      mocked.payslipsYear.mockResolvedValue(payslipsYear({ telework: "Old days" }));
      mocked.payslipView.mockResolvedValue(payslipView({ title: "Old title" }));
      mocked.yearPage.mockResolvedValue(yearPage({ total: "€100.00" }));
    });

    it.each([
      ["payslips", ["panel", "payslip", "year"]],
      ["tax_years", ["year"]],
      ["accounts", []],
    ] as const)("when data_changed names the %s table, refetches %j and keeps the rest", async (table, refreshed) => {
      renderWithQuery(<PayslipsProbe />);
      for (const [old] of Object.values(shown)) await screen.findByText(old);
      mocked.payslipsYear.mockResolvedValue(payslipsYear({ telework: "New days" }));
      mocked.payslipView.mockResolvedValue(payslipView({ title: "New title" }));
      mocked.yearPage.mockResolvedValue(yearPage({ total: "€200.00" }));

      await emitDataChanged([table]);

      for (const key of refreshed) expect(await screen.findByText(shown[key][1])).toBeTruthy();
      for (const key of Object.keys(shown) as (keyof typeof shown)[]) {
        if (!(refreshed as readonly string[]).includes(key)) expect(screen.getByText(shown[key][0])).toBeTruthy();
      }
    });
  });

  describe("year categories", () => {
    function CategoriesProbe() {
      useEvents();
      const { data: page } = useYearPage(2026, "");
      const { data: categories } = useYearCategories(2026);
      return (
        <>
          <p>Year: {page?.total}</p>
          <p>Categories: {categories?.map((c) => c.label).join(", ")}</p>
        </>
      );
    }

    const shown = {
      year: ["Year: €100.00", "Year: €200.00"],
      categories: ["Categories: Old category", "Categories: New category"],
    } as const;

    beforeEach(() => {
      mocked.yearPage.mockResolvedValue(yearPage({ total: "€100.00" }));
      mocked.yearCategories.mockResolvedValue([categoryChoice({ label: "Old category" })]);
    });

    it.each([
      ["hidden_items", ["year", "categories"]],
      ["tax_years", ["year", "categories"]],
      ["entries", ["year"]],
      ["accounts", []],
    ] as const)("when data_changed names the %s table, refetches %j and keeps the rest", async (table, refreshed) => {
      renderWithQuery(<CategoriesProbe />);
      for (const [old] of Object.values(shown)) await screen.findByText(old);
      mocked.yearPage.mockResolvedValue(yearPage({ total: "€200.00" }));
      mocked.yearCategories.mockResolvedValue([categoryChoice({ label: "New category" })]);

      await emitDataChanged([table]);

      for (const key of refreshed) expect(await screen.findByText(shown[key][1])).toBeTruthy();
      for (const key of Object.keys(shown) as (keyof typeof shown)[]) {
        if (!(refreshed as readonly string[]).includes(key)) expect(screen.getByText(shown[key][0])).toBeTruthy();
      }
    });
  });

  describe("today", () => {
    function TodayProbe() {
      useEvents();
      const { data: today } = useToday();
      return <p>Today: {today?.next?.title ?? "nothing open"}</p>;
    }

    beforeEach(() => {
      mocked.todayView.mockResolvedValue(todayView({ next: step({ title: "Old step" }) }));
    });

    it.each(["answers", "tax_years", "accounts", "vendors"])("refetches the next step when data_changed names the %s table", async (table) => {
      renderWithQuery(<TodayProbe />);
      await screen.findByText("Today: Old step");
      mocked.todayView.mockResolvedValue(todayView({ next: step({ title: "New step" }) }));

      await emitDataChanged([table]);

      expect(await screen.findByText("Today: New step")).toBeTruthy();
    });
  });

  describe("subscriptions", () => {
    function SubscriptionsProbe() {
      useEvents();
      const { data: view } = useSubscriptions("");
      const { data: detail } = useSubscription("sub-1");
      const { data: count } = useMatchCount(subscriptionForm());
      return (
        <>
          <p>List: {view?.summary}</p>
          <p>Detail: {detail?.name}</p>
          <p>Count: {count?.text}</p>
        </>
      );
    }

    const shown = {
      list: ["List: Old summary", "List: New summary"],
      detail: ["Detail: Old Name", "Detail: New Name"],
      count: ["Count: Old count", "Count: New count"],
    } as const;

    beforeEach(() => {
      mocked.subscriptionsView.mockResolvedValue(subscriptionsView({ summary: "Old summary" }));
      mocked.subscriptionDetail.mockResolvedValue(subscriptionDetail({ name: "Old Name" }));
      mocked.subscriptionMatchCount.mockResolvedValue({ text: "Old count", count: 1 });
    });

    it.each(["subscriptions", "plans", "decisions"])("refetches the list, the detail and the match count when data_changed names the %s table", async (table) => {
      renderWithQuery(<SubscriptionsProbe />);
      for (const [old] of Object.values(shown)) await screen.findByText(old);
      mocked.subscriptionsView.mockResolvedValue(subscriptionsView({ summary: "New summary" }));
      mocked.subscriptionDetail.mockResolvedValue(subscriptionDetail({ name: "New Name" }));
      mocked.subscriptionMatchCount.mockResolvedValue({ text: "New count", count: 2 });

      await emitDataChanged([table]);

      for (const [, fresh] of Object.values(shown)) expect(await screen.findByText(fresh)).toBeTruthy();
    });

    it("keeps the subscriptions when data_changed names an unrelated table", async () => {
      renderWithQuery(<SubscriptionsProbe />);
      for (const [old] of Object.values(shown)) await screen.findByText(old);
      mocked.subscriptionsView.mockResolvedValue(subscriptionsView({ summary: "New summary" }));

      await emitDataChanged(["accounts"]);

      expect(screen.getByText("List: Old summary")).toBeTruthy();
    });
  });

  describe("today", () => {
    function TodayProbe() {
      useEvents();
      const { data } = useToday();
      return <p>Today: {data?.next?.title ?? "nothing open"}</p>;
    }

    const open = (title: string) => mocked.todayView.mockResolvedValue(todayView({ next: step({ title }) }));

    it.each(["decisions", "documents", "statements", "subscriptions"])("refetches Today when data_changed names the %s table", async (table) => {
      open("Old step");
      renderWithQuery(<TodayProbe />);
      await screen.findByText("Today: Old step");
      open("New step");

      await emitDataChanged([table]);

      expect(await screen.findByText("Today: New step")).toBeTruthy();
    });

    it("refetches Today and every other query when the settings change", async () => {
      function EverythingProbe() {
        useEvents();
        const { data: today } = useToday();
        const { data: subscriptions } = useSubscriptions("");
        const { data: year } = useYearPage(2026, "");
        return (
          <>
            <p>Today: {today?.next?.title ?? "nothing open"}</p>
            <p>Subscriptions: {subscriptions?.summary}</p>
            <p>Year: {year?.total}</p>
          </>
        );
      }
      open("Old step");
      mocked.subscriptionsView.mockResolvedValue(subscriptionsView({ summary: "Old summary" }));
      mocked.yearPage.mockResolvedValue(yearPage({ total: "€100.00" }));
      renderWithQuery(<EverythingProbe />);
      await screen.findByText("Today: Old step");
      await screen.findByText("Subscriptions: Old summary");
      await screen.findByText("Year: €100.00");
      open("New step");
      mocked.subscriptionsView.mockResolvedValue(subscriptionsView({ summary: "New summary" }));
      mocked.yearPage.mockResolvedValue(yearPage({ total: "€200.00" }));
      await waitFor(() => expect(mocked.listen.mock.calls.some(([name]) => name === "settings_changed")).toBe(true));
      const handler = mocked.listen.mock.calls.find(([name]) => name === "settings_changed")![1] as (payload: null) => void;

      await act(async () => handler(null));

      expect(await screen.findByText("Today: New step")).toBeTruthy();
      expect(await screen.findByText("Subscriptions: New summary")).toBeTruthy();
      expect(await screen.findByText("Year: €200.00")).toBeTruthy();
    });
  });

  describe("commands", () => {
    function CommandsProbe() {
      useEvents();
      const { data } = useCommands();
      return <p>Shortcuts: {data?.map((c) => c.shortcut ?? "none").join(", ")}</p>;
    }

    it("refetches the commands when commands_changed arrives", async () => {
      mocked.listCommands.mockResolvedValue([{ id: "go:today", title: "Go to Today", shortcut: "⌘1", custom: false }]);
      renderWithQuery(<CommandsProbe />);
      await screen.findByText("Shortcuts: ⌘1");
      mocked.listCommands.mockResolvedValue([{ id: "go:today", title: "Go to Today", shortcut: "⌘⇧T", custom: true }]);
      await waitFor(() => expect(mocked.listen.mock.calls.some(([name]) => name === "commands_changed")).toBe(true));
      const handler = mocked.listen.mock.calls.find(([name]) => name === "commands_changed")![1] as (payload: null) => void;

      await act(async () => handler(null));

      expect(await screen.findByText("Shortcuts: ⌘⇧T")).toBeTruthy();
    });
  });

  describe("file check", () => {
    function IntegrityProbe() {
      useEvents();
      const { data: check } = useLastFileCheck();
      const { data: groups } = useStatus();
      return (
        <>
          <p>Problems: {check ? check.problems.length : "none run"}</p>
          <p>Status: {groups?.map((g) => g.title).join(", ")}</p>
        </>
      );
    }

    it("refetches the last check and the status when files_checked arrives", async () => {
      mocked.filesLastCheck.mockResolvedValue(null);
      mocked.statusView.mockResolvedValue([statusGroup({ title: "Old group" })]);
      renderWithQuery(<IntegrityProbe />);
      await screen.findByText("Problems: none run");
      await screen.findByText("Status: Old group");
      mocked.filesLastCheck.mockResolvedValue(fileCheck({ problems: [fileProblem()] }));
      mocked.statusView.mockResolvedValue([statusGroup({ title: "New group" })]);
      await waitFor(() => expect(mocked.listen.mock.calls.some(([name]) => name === "files_checked")).toBe(true));
      const handler = mocked.listen.mock.calls.find(([name]) => name === "files_checked")![1] as (payload: null) => void;

      await act(async () => handler(null));

      expect(await screen.findByText("Problems: 1")).toBeTruthy();
      expect(await screen.findByText("Status: New group")).toBeTruthy();
    });

    it("refetches the status when data_changed names the documents table", async () => {
      mocked.statusView.mockResolvedValue([statusGroup({ title: "Old group" })]);
      renderWithQuery(<IntegrityProbe />);
      await screen.findByText("Status: Old group");
      mocked.statusView.mockResolvedValue([statusGroup({ title: "New group" })]);

      await emitDataChanged(["documents"]);

      expect(await screen.findByText("Status: New group")).toBeTruthy();
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
