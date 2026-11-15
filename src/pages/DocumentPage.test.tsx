import { act, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { useStore } from "../state/store";
import { documentView, historyEntry } from "../test/documents";
import { renderWithQuery } from "../test/renderWithQuery";
import { DocumentPage } from "./DocumentPage";

const mocked = vi.mocked(api, true);

beforeEach(() => {
  vi.clearAllMocks();
  useStore.setState({ toasts: [], confirmation: null, pdfCovers: 0 });
  mocked.documentHistory.mockResolvedValue([]);
  mocked.documentStep.mockResolvedValue("Done");
});

describe("DocumentPage", () => {
  it("shows the PDF preview and the form of the document in a sidebar named after it, with the file path only as a tooltip on the preview", async () => {
    mocked.documentView.mockResolvedValue(documentView({ missing: ["vendor"] }));
    renderWithQuery(<DocumentPage documentId="doc-1" />);

    const side = await screen.findByRole("complementary", { name: "Demo Cloud Ltd · DEM-2026-1109" });
    expect(within(side).getByText("Missing: vendor")).toBeTruthy();
    expect(screen.queryByText("Documents/Inbox/demo-cloud-1109.pdf")).toBeNull();
    expect(screen.getByTitle("Documents/Inbox/demo-cloud-1109.pdf")).toBeTruthy();
    expect(mocked.documentView).toHaveBeenCalledWith("doc-1");
    await waitFor(() => expect(mocked.pdfViewShow).toHaveBeenCalledWith({ kind: "document", id: "doc-1" }, expect.anything()));
  });

  it("shows the selection hint and the PDF preview while the document loads", () => {
    mocked.documentView.mockReturnValue(new Promise(() => {}));
    renderWithQuery(<DocumentPage documentId="doc-1" />);

    expect(screen.getByText("Text in the PDF can be selected and copied")).toBeTruthy();
    expect(screen.getByRole("complementary", { name: "Document" })).toBeTruthy();
    expect(mocked.pdfViewShow).toHaveBeenCalledWith({ kind: "document", id: "doc-1" }, expect.anything());
    expect(screen.queryByText("Every required fact is set")).toBeNull();
  });

  describe("finished document", () => {
    beforeEach(() => {
      mocked.documentView.mockResolvedValue(documentView({ state: "finished", finishedOn: "12 Nov 2026" }));
    });

    it("shows the locked banner", async () => {
      renderWithQuery(<DocumentPage documentId="doc-1" />);

      expect((await screen.findByRole("status")).textContent).toContain("Finished on 12 Nov 2026. Details are locked");
    });

    it("unlocks after the owner confirms", async () => {
      renderWithQuery(<DocumentPage documentId="doc-1" />);

      await userEvent.click(await screen.findByRole("button", { name: "Unlock…" }));
      expect(useStore.getState().confirmation?.title).toBe("Unlock this document?");
      expect(mocked.documentStep).not.toHaveBeenCalled();
      act(() => useStore.getState().confirmation?.answer(true));

      await waitFor(() => expect(mocked.documentStep).toHaveBeenCalledWith("doc-1", { kind: "unlock" }));
    });

    it("keeps the document locked when the owner cancels", async () => {
      renderWithQuery(<DocumentPage documentId="doc-1" />);

      await userEvent.click(await screen.findByRole("button", { name: "Unlock…" }));
      act(() => useStore.getState().confirmation?.answer(false));

      expect(mocked.documentStep).not.toHaveBeenCalled();
    });
  });

  describe("excluded document", () => {
    it("shows the excluded banner with the reason and restores", async () => {
      mocked.documentView.mockResolvedValue(documentView({ state: "excluded", excludedOn: "2 Nov 2026", excludeReason: "Duplicate" }));
      renderWithQuery(<DocumentPage documentId="doc-1" />);

      expect((await screen.findByRole("status")).textContent).toContain("Excluded on 2 Nov 2026: Duplicate.");
      await userEvent.click(screen.getByRole("button", { name: "Restore" }));

      expect(mocked.documentStep).toHaveBeenCalledWith("doc-1", { kind: "restore" });
    });

    it("leaves out the reason when there is none", async () => {
      mocked.documentView.mockResolvedValue(documentView({ state: "excluded", excludedOn: "2 Nov 2026", excludeReason: null }));
      renderWithQuery(<DocumentPage documentId="doc-1" />);

      expect((await screen.findByRole("status")).textContent).toContain("Excluded on 2 Nov 2026. It counts nowhere");
    });
  });

  it("shows no banner for a draft", async () => {
    mocked.documentView.mockResolvedValue(documentView());
    renderWithQuery(<DocumentPage documentId="doc-1" />);

    await screen.findByText("Every required fact is set");
    expect(screen.queryByRole("status")).toBeNull();
  });

  describe("history", () => {
    const summary = () => screen.getByText("History", { selector: "summary" });

    it("is folded away until its heading is clicked", async () => {
      mocked.documentView.mockResolvedValue(documentView());
      renderWithQuery(<DocumentPage documentId="doc-1" />);
      await screen.findByText("Every required fact is set");

      expect(summary().closest("details")?.open).toBe(false);
      await userEvent.click(summary());

      expect(summary().closest("details")?.open).toBe(true);
    });

    it("lists the entries of the document with the write-once note", async () => {
      mocked.documentView.mockResolvedValue(documentView());
      mocked.documentHistory.mockResolvedValue([historyEntry({ text: "Finished", when: "13 Nov 2026, 15:00" }), historyEntry({ text: "Added", when: "12 Nov 2026, 09:10" })]);
      renderWithQuery(<DocumentPage documentId="doc-1" />);
      await userEvent.click(await screen.findByText("History", { selector: "summary" }));

      const list = screen.getByRole("list", { name: "Changes" });
      const items = await within(list).findAllByRole("listitem");
      expect(items.map((li) => li.textContent)).toEqual(["Finished13 Nov 2026, 15:00", "Added12 Nov 2026, 09:10"]);
      expect(screen.getByText("History lines are written once and never edited.")).toBeTruthy();
      expect(mocked.documentHistory).toHaveBeenCalledWith("doc-1");
    });
  });

  describe("excluding", () => {
    it("opens the Exclude sheet from Exclude Document…", async () => {
      mocked.documentView.mockResolvedValue(documentView());
      renderWithQuery(<DocumentPage documentId="doc-1" />);

      await userEvent.click(await screen.findByRole("button", { name: "Exclude Document…" }));

      expect(screen.getByRole("heading", { name: "Exclude Demo Cloud Ltd · DEM-2026-1109?" })).toBeTruthy();
    });

    it("closes the sheet on Cancel", async () => {
      mocked.documentView.mockResolvedValue(documentView());
      renderWithQuery(<DocumentPage documentId="doc-1" />);
      await userEvent.click(await screen.findByRole("button", { name: "Exclude Document…" }));

      await userEvent.click(screen.getByRole("button", { name: "Cancel" }));

      expect(screen.queryByRole("heading", { name: /^Exclude / })).toBeNull();
    });
  });

  it("says there is no such document, without a form, preview or error toast, when it is not found", async () => {
    mocked.documentView.mockRejectedValue({ kind: "not_found", message: "Document not found", code: null });
    useStore.setState({ toasts: [] });
    renderWithQuery(<DocumentPage documentId="doc-missing" />);

    expect(await screen.findByText("No such document")).toBeTruthy();
    expect(screen.getByText("It may have been deleted, or the link is old.")).toBeTruthy();
    expect(screen.queryByLabelText("Vendor")).toBeNull();
    expect(screen.queryByText("Text in the PDF can be selected and copied")).toBeNull();
    expect(useStore.getState().toasts).toEqual([]);
  });
});
