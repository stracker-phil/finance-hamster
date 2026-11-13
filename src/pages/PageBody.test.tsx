import { render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { renderWithQuery } from "../test/renderWithQuery";
import { reviewQueue } from "../test/review";
import { PageBody } from "./PageBody";

const mocked = vi.mocked(api, true);

beforeEach(() => {
  vi.clearAllMocks();
});

describe("PageBody", () => {
  it("shows the PDF preview of the document on a document page", () => {
    render(<PageBody route={{ page: "document", documentId: "d1", year: null }} />);

    expect(mocked.pdfViewShow).toHaveBeenCalledWith({ kind: "document", id: "d1" }, expect.anything());
  });

  it("shows the Review page on the review route", async () => {
    mocked.reviewQueue.mockResolvedValue(reviewQueue());

    renderWithQuery(<PageBody route={{ page: "review" }} />);

    expect(await screen.findByRole("heading", { name: "Example Bookshop" })).toBeTruthy();
  });

  it("shows no PDF preview on a page without a document", () => {
    render(<PageBody route={{ page: "documents" }} />);

    expect(screen.getByText("No documents")).toBeTruthy();
    expect(mocked.pdfViewShow).not.toHaveBeenCalled();
  });
});
