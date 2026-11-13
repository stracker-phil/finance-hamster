import { screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { useStore } from "../state/store";
import { documentView } from "../test/documents";
import { renderWithQuery } from "../test/renderWithQuery";
import { DocumentPage } from "./DocumentPage";

const mocked = vi.mocked(api, true);

beforeEach(() => {
  vi.clearAllMocks();
});

describe("DocumentPage", () => {
  it("shows the file path and the form of the document", async () => {
    mocked.documentView.mockResolvedValue(documentView({ missing: ["vendor"] }));
    renderWithQuery(<DocumentPage documentId="doc-1" />);

    expect(await screen.findByText("Documents/Inbox/demo-cloud-1109.pdf")).toBeTruthy();
    expect(screen.getByText("Missing: vendor")).toBeTruthy();
    expect(mocked.documentView).toHaveBeenCalledWith("doc-1");
  });

  it("shows the selection hint and the PDF preview while the document loads", () => {
    mocked.documentView.mockReturnValue(new Promise(() => {}));
    renderWithQuery(<DocumentPage documentId="doc-1" />);

    expect(screen.getByText("Text in the PDF can be selected and copied")).toBeTruthy();
    expect(mocked.pdfViewShow).toHaveBeenCalledWith({ kind: "document", id: "doc-1" }, expect.anything());
    expect(screen.queryByText("Every required fact is set")).toBeNull();
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
