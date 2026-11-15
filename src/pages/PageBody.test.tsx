import { screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { archivedDocument, documentGroup, historyEntry, otherView, statementView } from "../test/documents";
import { payslipView } from "../test/payslips";
import { renderWithQuery } from "../test/renderWithQuery";
import { accountLines } from "../test/statements";
import { reviewQueue } from "../test/review";
import { subscriptionsView } from "../test/subscriptions";
import { step, todayView } from "../test/today";
import { yearPage } from "../test/year";
import { PageBody } from "./PageBody";

const mocked = vi.mocked(api, true);

beforeEach(() => {
  vi.clearAllMocks();
  mocked.documentsList.mockResolvedValue([]);
  mocked.yearLog.mockResolvedValue([]);
  mocked.archiveList.mockResolvedValue([]);
});

describe("PageBody", () => {
  it("shows the Today page on the today route", async () => {
    mocked.todayView.mockResolvedValue(todayView({ next: step({ title: "3 invoices need a PDF" }) }));

    renderWithQuery(<PageBody route={{ page: "today" }} />);

    expect(await screen.findByRole("heading", { name: "3 invoices need a PDF" })).toBeTruthy();
  });

  it("shows the PDF preview of the document on a document page", () => {
    renderWithQuery(<PageBody route={{ page: "document", documentId: "d1", year: null }} />);

    expect(mocked.pdfViewShow).toHaveBeenCalledWith({ kind: "document", id: "d1" }, expect.anything());
  });

  it("shows the Documents page on the documents route", async () => {
    mocked.documentsList.mockResolvedValue([documentGroup()]);

    renderWithQuery(<PageBody route={{ page: "documents" }} />);

    expect(await screen.findByRole("button", { name: /Demo Cloud Ltd/ })).toBeTruthy();
    expect(screen.getByRole("textbox", { name: "Search documents" })).toBeTruthy();
  });

  it.each([
    ["statements", "Import Statement…"],
    ["other", "Add Document…"],
  ] as const)("shows the %s tab of Documents when the route names it", async (tab, action) => {
    renderWithQuery(<PageBody route={{ page: "documents", tab }} />);

    expect(await screen.findByRole("button", { name: action })).toBeTruthy();
  });

  it("shows a statement on the statement route", async () => {
    mocked.statementView.mockResolvedValue(statementView());

    renderWithQuery(<PageBody route={{ page: "statement", statementId: "st-10" }} />);

    expect(await screen.findByRole("heading", { name: "Private · Nr. 010/2026" })).toBeTruthy();
    expect(mocked.statementView).toHaveBeenCalledWith("st-10");
  });

  it("shows an other document on the other route", async () => {
    mocked.otherView.mockResolvedValue(otherView());

    renderWithQuery(<PageBody route={{ page: "other", otherId: "other-1" }} />);

    expect(await screen.findByLabelText("Title")).toHaveProperty("value", "Demo Lease");
    expect(mocked.otherView).toHaveBeenCalledWith("other-1");
  });

  it("shows a payslip on the payslip route", async () => {
    mocked.payslipView.mockResolvedValue(payslipView());

    renderWithQuery(<PageBody route={{ page: "payslip", payslipId: "ps-7", year: 2026 }} />);

    expect(await screen.findByRole("heading", { name: "Payslip July 2026" })).toBeTruthy();
    expect(mocked.payslipView).toHaveBeenCalledWith("ps-7");
  });

  it("shows the lines of an account on the account route", async () => {
    mocked.accountLines.mockResolvedValue(accountLines({ id: "acc-2" }));

    renderWithQuery(<PageBody route={{ page: "account", accountId: "acc-2", statementId: "st-1" }} />);

    expect(await screen.findByRole("region", { name: "Nr. 010/2026" })).toBeTruthy();
    expect(mocked.accountLines).toHaveBeenCalledWith("acc-2", "");
  });

  it("shows the Subscriptions page on the subscriptions route", async () => {
    mocked.subscriptionsView.mockResolvedValue(subscriptionsView());

    renderWithQuery(<PageBody route={{ page: "subscriptions" }} />);

    expect(await screen.findByRole("button", { name: /Lumen AI/ })).toBeTruthy();
    expect(screen.getByText("1 active · €20.00 a month")).toBeTruthy();
  });

  it("shows the Review page on the review route", async () => {
    mocked.reviewQueue.mockResolvedValue(reviewQueue());

    renderWithQuery(<PageBody route={{ page: "review" }} />);

    expect(await screen.findByRole("heading", { name: "Example Bookshop" })).toBeTruthy();
  });

  it("shows the Log of the year on the log route", async () => {
    mocked.yearLog.mockResolvedValue([historyEntry({ document: "Example Market" })]);

    renderWithQuery(<PageBody route={{ page: "log", year: 2026 }} />);

    expect(await screen.findByRole("button", { name: "Example Market" })).toBeTruthy();
    expect(mocked.yearLog).toHaveBeenCalledWith(2026, "");
  });

  it("shows the tax year page on the year route", async () => {
    mocked.yearPage.mockResolvedValue(yearPage({ total: "€1,228.40" }));

    renderWithQuery(<PageBody route={{ page: "year", year: 2026 }} />);

    expect(await screen.findByText("€1,228.40")).toBeTruthy();
    expect(screen.getByText("Work costs 2026")).toBeTruthy();
  });

  it("shows the Archive of the year on the archive route", async () => {
    mocked.archiveList.mockResolvedValue([archivedDocument()]);

    renderWithQuery(<PageBody route={{ page: "archive", year: 2026 }} />);

    expect(await screen.findByText("Sample Books")).toBeTruthy();
    expect(mocked.archiveList).toHaveBeenCalledWith(2026);
  });

  it("shows no PDF preview on a page without a document", async () => {
    renderWithQuery(<PageBody route={{ page: "documents" }} />);

    expect(await screen.findByText("No documents")).toBeTruthy();
    expect(mocked.pdfViewShow).not.toHaveBeenCalled();
  });
});
