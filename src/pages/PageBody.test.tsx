import { screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { archivedDocument, documentGroup, historyEntry } from "../test/documents";
import { renderWithQuery } from "../test/renderWithQuery";
import { reviewQueue } from "../test/review";
import { PageBody } from "./PageBody";

const mocked = vi.mocked(api, true);

beforeEach(() => {
  vi.clearAllMocks();
  mocked.documentsList.mockResolvedValue([]);
  mocked.yearLog.mockResolvedValue([]);
  mocked.archiveList.mockResolvedValue([]);
});

describe("PageBody", () => {
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

  it("shows the Review page on the review route", async () => {
    mocked.reviewQueue.mockResolvedValue(reviewQueue());

    renderWithQuery(<PageBody route={{ page: "review" }} />);

    expect(await screen.findByRole("heading", { name: "Example Bookshop" })).toBeTruthy();
  });

  it("shows the Log of the year on the log route", async () => {
    mocked.yearLog.mockResolvedValue([historyEntry({ document: "Example Market" })]);

    renderWithQuery(<PageBody route={{ page: "log", year: 2026 }} />);

    expect(await screen.findByRole("button", { name: "Example Market" })).toBeTruthy();
    expect(mocked.yearLog).toHaveBeenCalledWith(2026);
  });

  it("shows the Archive of the year on the archive route", async () => {
    mocked.archiveList.mockResolvedValue([archivedDocument()]);

    renderWithQuery(<PageBody route={{ page: "archive", year: 2026 }} />);

    expect(await screen.findByText("Documents/2026/sample-books.pdf")).toBeTruthy();
    expect(mocked.archiveList).toHaveBeenCalledWith(2026);
  });

  it("shows no PDF preview on a page without a document", async () => {
    renderWithQuery(<PageBody route={{ page: "documents" }} />);

    expect(await screen.findByText("No documents")).toBeTruthy();
    expect(mocked.pdfViewShow).not.toHaveBeenCalled();
  });
});
