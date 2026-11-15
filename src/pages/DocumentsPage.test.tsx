import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { HOME } from "../state/route";
import { useStore } from "../state/store";
import { accountStatements, documentGroup, documentRow, otherGroup } from "../test/documents";
import { renderWithQuery } from "../test/renderWithQuery";
import { DocumentsPage } from "./DocumentsPage";

const mocked = vi.mocked(api, true);

const inbox = () =>
  documentGroup({ title: "Inbox", year: null, count: "1 document", total: null, documents: [documentRow({ id: "doc-2", title: "Example Market", sub: "no payment date yet", state: "draft", anv: "" })] });

beforeEach(() => {
  vi.clearAllMocks();
  useStore.setState({ route: HOME, back: [], forward: [], filter: "", toasts: [], documentsTab: "invoices", addPreview: null, importPreview: null });
  mocked.documentsList.mockResolvedValue([]);
  mocked.statementsList.mockResolvedValue([]);
  mocked.othersList.mockResolvedValue([]);
  mocked.payslipsList.mockResolvedValue([]);
  mocked.documentAddChoose.mockResolvedValue(null);
  mocked.statementImportChoose.mockResolvedValue(null);
  mocked.otherAddPdf.mockResolvedValue(null);
});

const tabButton = (name: string) => screen.getByRole("button", { name });

describe("DocumentsPage", () => {
  describe("tabs", () => {
    it("offers Invoices, Statements, Payslips and Other as one group with Invoices pressed first", async () => {
      renderWithQuery(<DocumentsPage />);

      const group = screen.getByRole("group", { name: "Kind of document" });
      expect(within(group).getAllByRole("button").map((b) => b.textContent)).toEqual(["Invoices", "Statements", "Payslips", "Other"]);
      expect(tabButton("Invoices").getAttribute("aria-pressed")).toBe("true");
      await screen.findByText("No documents");
    });

    it("opens the tab the route asks for", async () => {
      mocked.othersList.mockResolvedValue([otherGroup()]);

      renderWithQuery(<DocumentsPage tab="other" />);

      expect(await screen.findByRole("button", { name: /Demo Lease/ })).toBeTruthy();
      expect(tabButton("Other").getAttribute("aria-pressed")).toBe("true");
      expect(tabButton("Invoices").getAttribute("aria-pressed")).toBe("false");
    });

    it("opens the tab opened last when the route names none", async () => {
      useStore.setState({ documentsTab: "statements" });
      mocked.statementsList.mockResolvedValue([accountStatements()]);

      renderWithQuery(<DocumentsPage />);

      expect(await screen.findByRole("region", { name: "Private" })).toBeTruthy();
      expect(tabButton("Statements").getAttribute("aria-pressed")).toBe("true");
    });

    it.each([
      ["Statements", "statements"],
      ["Payslips", "payslips"],
      ["Other", "other"],
      ["Invoices", "invoices"],
    ] as const)("remembers %s and navigates to it when its tab is clicked", async (name, tab) => {
      renderWithQuery(<DocumentsPage tab={tab === "invoices" ? "other" : "invoices"} />);

      await userEvent.click(tabButton(name));

      expect(useStore.getState().documentsTab).toBe(tab);
      expect(useStore.getState().route).toEqual({ page: "documents", tab });
    });
  });

  describe("empty tabs", () => {
    it("explains how to add a statement and asks for one with Import Statement…", async () => {
      renderWithQuery(<DocumentsPage tab="statements" />);

      expect(await screen.findByText("No statements yet. Drop a bank statement PDF onto the window, or import one.")).toBeTruthy();
      await userEvent.click(screen.getByRole("button", { name: "Import Statement…" }));

      await waitFor(() => expect(mocked.statementImportChoose).toHaveBeenCalled());
    });

    it("explains how to keep another document and asks for a PDF with Add Document…", async () => {
      renderWithQuery(<DocumentsPage tab="other" />);

      expect(await screen.findByText("Keep any PDF here: a contract, a policy, a tax assessment.")).toBeTruthy();
      await userEvent.click(screen.getByRole("button", { name: "Add Document…" }));

      await waitFor(() => expect(mocked.otherAddPdf).toHaveBeenCalled());
    });

    it("opens the kept document, saying so with Undo, once a PDF was chosen with Add Document…", async () => {
      mocked.otherAddPdf.mockResolvedValue({ id: "other-new", label: "Add Demo Lease" });
      renderWithQuery(<DocumentsPage tab="other" />);

      await userEvent.click(await screen.findByRole("button", { name: "Add Document…" }));

      await waitFor(() => expect(useStore.getState().route).toEqual({ page: "other", otherId: "other-new" }));
      expect(useStore.getState().toasts.map((t) => t.message)).toEqual(["Add Demo Lease"]);
    });

    it("reports a PDF that cannot be kept as an error toast and stays on the tab", async () => {
      mocked.otherAddPdf.mockRejectedValue({ kind: "invalid_input", message: "This PDF is already kept as Demo Lease", code: null });
      renderWithQuery(<DocumentsPage tab="other" />);

      await userEvent.click(await screen.findByRole("button", { name: "Add Document…" }));

      await waitFor(() => expect(useStore.getState().toasts.map((t) => t.message)).toEqual(["This PDF is already kept as Demo Lease"]));
      expect(useStore.getState().route).toEqual(HOME);
    });

    it.each([
      ["statements", "Import Statement…"],
      ["other", "Add Document…"],
    ] as const)("says nothing matches, without the add button, when a search finds no %s", async (tab, action) => {
      renderWithQuery(<DocumentsPage tab={tab} />);
      await screen.findByRole("button", { name: action });

      await userEvent.type(screen.getByRole("textbox", { name: "Search documents" }), "xyz");

      expect(await screen.findByText("Nothing matches “xyz”, so it isn't here yet.")).toBeTruthy();
      expect(screen.queryByRole("button", { name: action })).toBeNull();
    });
  });

  describe("search on the other tabs", () => {
    it("asks for the statements matching the typed text", async () => {
      mocked.statementsList.mockResolvedValue([accountStatements()]);
      renderWithQuery(<DocumentsPage tab="statements" />);
      await screen.findByRole("region", { name: "Private" });

      await userEvent.type(screen.getByRole("textbox", { name: "Search documents" }), "nov");

      await waitFor(() => expect(mocked.statementsList).toHaveBeenLastCalledWith("nov"));
    });

    it("asks for the other documents matching the typed text", async () => {
      mocked.othersList.mockResolvedValue([otherGroup()]);
      renderWithQuery(<DocumentsPage tab="other" />);
      await screen.findByRole("region", { name: "2026" });

      await userEvent.type(screen.getByRole("textbox", { name: "Search documents" }), "lease");

      await waitFor(() => expect(mocked.othersList).toHaveBeenLastCalledWith("lease"));
    });
  });

  describe("groups", () => {
    beforeEach(() => {
      mocked.documentsList.mockResolvedValue([inbox(), documentGroup({ documents: [documentRow(), documentRow({ id: "doc-3", title: "Sample Books", state: "excluded", anv: "€0.00" })], count: "2 documents", total: "€121.23" })]);
    });

    it("shows each group with its count and total", async () => {
      renderWithQuery(<DocumentsPage />);

      const year = await screen.findByRole("region", { name: "Tax year 2026" });
      expect(within(year).getByText("2 documents")).toBeTruthy();
      expect(within(year).getByText("€121.23", { selector: ".docs-group__total" })).toBeTruthy();
      expect(within(screen.getByRole("region", { name: "Inbox" })).getByText("1 document")).toBeTruthy();
    });

    it("shows no total for a group without one", async () => {
      mocked.documentsList.mockResolvedValue([documentGroup({ total: null })]);
      renderWithQuery(<DocumentsPage />);

      const year = await screen.findByRole("region", { name: "Tax year 2026" });

      expect(within(year).getByText("1 document")).toBeTruthy();
      expect(year.querySelector(".docs-group__total")).toBeNull();
    });

    it("shows each row with its title, sub line, state and ANV amount", async () => {
      renderWithQuery(<DocumentsPage />);

      const row = await screen.findByRole("button", { name: /Demo Cloud Ltd/ });
      expect(within(row).getByText("10 Nov · KZ 169 Digital work equipment")).toBeTruthy();
      expect(within(row).getByText("Finished")).toBeTruthy();
      expect(within(row).getByText("€121.23")).toBeTruthy();
      expect(within(screen.getByRole("button", { name: /Example Market/ })).getByText("Draft")).toBeTruthy();
      expect(within(screen.getByRole("button", { name: /Sample Books/ })).getByText("Excluded")).toBeTruthy();
    });

    it("opens the tax year page from a year's title, while the Inbox title is no button", async () => {
      renderWithQuery(<DocumentsPage />);

      await userEvent.click(await screen.findByRole("button", { name: "Tax year 2026" }));

      expect(useStore.getState().route).toEqual({ page: "year", year: 2026 });
      expect(screen.queryByRole("button", { name: "Inbox" })).toBeNull();
    });

    it.each([
      ["Log", "log"],
      ["Archive", "archive"],
    ] as const)("opens the %s of a tax year from its button", async (name, page) => {
      renderWithQuery(<DocumentsPage />);
      const year = await screen.findByRole("region", { name: "Tax year 2026" });

      await userEvent.click(within(year).getByRole("button", { name }));

      expect(useStore.getState().route).toEqual({ page, year: 2026 });
    });

    it("offers no Log or Archive for the Inbox", async () => {
      renderWithQuery(<DocumentsPage />);

      const inboxGroup = await screen.findByRole("region", { name: "Inbox" });

      expect(within(inboxGroup).queryByRole("button", { name: "Log" })).toBeNull();
      expect(within(inboxGroup).queryByRole("button", { name: "Archive" })).toBeNull();
    });

    it("opens a document when its row is clicked", async () => {
      renderWithQuery(<DocumentsPage />);

      await userEvent.click(await screen.findByRole("button", { name: /Example Market/ }));

      expect(useStore.getState().route).toEqual({ page: "document", documentId: "doc-2", year: null });
    });
  });

  describe("search", () => {
    it("lists the documents matching the typed text", async () => {
      mocked.documentsList.mockResolvedValue([documentGroup()]);
      renderWithQuery(<DocumentsPage />);
      await screen.findByRole("button", { name: /Demo Cloud Ltd/ });
      mocked.documentsList.mockResolvedValue([documentGroup({ documents: [documentRow({ id: "doc-3", title: "Sample Books" })], count: "1 document" })]);

      await userEvent.type(screen.getByRole("textbox", { name: "Search documents" }), "books");

      expect(mocked.documentsList).toHaveBeenLastCalledWith("books");
      expect(await screen.findByRole("button", { name: /Sample Books/ })).toBeTruthy();
      expect(screen.queryByRole("button", { name: /Demo Cloud Ltd/ })).toBeNull();
    });

    it("shares the header filter: it fills the big field and is the query", async () => {
      useStore.setState({ filter: "cloud" });
      renderWithQuery(<DocumentsPage />);
      await screen.findByText(/Nothing matches “cloud”/);

      expect(mocked.documentsList).toHaveBeenLastCalledWith("cloud");
      expect(screen.getByRole<HTMLInputElement>("textbox", { name: "Search documents" }).value).toBe("cloud");
    });

    it("sets the shared filter when typing in the big field", async () => {
      renderWithQuery(<DocumentsPage />);
      await screen.findByText("No documents");

      await userEvent.type(screen.getByRole("textbox", { name: "Search documents" }), "books");

      expect(useStore.getState().filter).toBe("books");
      expect(mocked.documentsList).toHaveBeenLastCalledWith("books");
    });

    it("says nothing matches, with the trimmed text, when a search finds none", async () => {
      renderWithQuery(<DocumentsPage />);
      await screen.findByText("No documents");

      await userEvent.type(screen.getByRole("textbox", { name: "Search documents" }), " xyz ");

      expect(await screen.findByText("Nothing matches “xyz”, so it isn't here yet.")).toBeTruthy();
      expect(screen.queryByText("No documents")).toBeNull();
    });
  });

  it("explains how to add a document when there are none", async () => {
    renderWithQuery(<DocumentsPage />);

    expect(await screen.findByText("No documents")).toBeTruthy();
    expect(screen.getByText("Drop an invoice PDF onto the window, or choose one.")).toBeTruthy();
  });

  it("asks for an invoice PDF when Choose File… is clicked on the empty page", async () => {
    mocked.documentAddChoose.mockResolvedValue(null);
    renderWithQuery(<DocumentsPage />);

    await userEvent.click(await screen.findByRole("button", { name: "Choose File…" }));

    await waitFor(() => expect(mocked.documentAddChoose).toHaveBeenCalled());
  });

  it("offers no Choose File… once documents exist", async () => {
    mocked.documentsList.mockResolvedValue([documentGroup()]);
    renderWithQuery(<DocumentsPage />);

    await screen.findByRole("button", { name: /Demo Cloud Ltd/ });
    expect(screen.queryByRole("button", { name: "Choose File…" })).toBeNull();
  });
});
