import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { PdfPreview } from "./PdfPreview";

const mocked = vi.mocked(api, true);

let rect = { left: 10, top: 20, width: 300, height: 400 };

beforeEach(() => {
  vi.clearAllMocks();
  for (const fn of [mocked.pdfViewShow, mocked.pdfViewMove, mocked.pdfViewHide]) fn.mockReset().mockResolvedValue(undefined);
  rect = { left: 10, top: 20, width: 300, height: 400 };
  vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockImplementation(() => rect as DOMRect);
  vi.spyOn(window, "requestAnimationFrame").mockImplementation((callback) => {
    callback(0);
    return 1;
  });
});

afterEach(() => {
  vi.restoreAllMocks();
});

function deferred() {
  let resolve!: () => void;
  const promise = new Promise<void>((r) => {
    resolve = r;
  });
  return { promise, resolve };
}

describe("PdfPreview", () => {
  it("shows the PDF of its source in the frame of its box on mount", () => {
    render(<PdfPreview source={{ kind: "statement", id: "s1" }} />);

    expect(mocked.pdfViewShow).toHaveBeenCalledWith({ kind: "statement", id: "s1" }, { x: 10, y: 20, width: 300, height: 400 });
  });

  it("hides the native view on unmount", () => {
    const { unmount } = render(<PdfPreview source={{ kind: "statement", id: "s1" }} />);
    expect(mocked.pdfViewHide).not.toHaveBeenCalled();

    unmount();

    expect(mocked.pdfViewHide).toHaveBeenCalled();
  });

  it("hides the old PDF before it shows the new one when the source changes", () => {
    const { rerender } = render(<PdfPreview source={{ kind: "document", id: "d1" }} />);

    rerender(<PdfPreview source={{ kind: "document", id: "d2" }} />);

    expect(mocked.pdfViewShow).toHaveBeenLastCalledWith({ kind: "document", id: "d2" }, expect.anything());
    expect(mocked.pdfViewHide.mock.invocationCallOrder[0]).toBeLessThan(mocked.pdfViewShow.mock.invocationCallOrder[1]);
  });

  it("shows no message while the PDF can be shown", async () => {
    render(<PdfPreview source={{ kind: "statement", id: "s1" }} />);

    await waitFor(() => expect(mocked.pdfViewMove).toHaveBeenCalled());
    expect(screen.queryByText("This PDF can't be shown.")).toBeNull();
  });

  it("shows a message when the PDF cannot be shown", async () => {
    mocked.pdfViewShow.mockRejectedValue({ kind: "not_found", message: "No such PDF", code: null });

    render(<PdfPreview source={{ kind: "statement", id: "gone" }} />);

    expect(await screen.findByText("This PDF can't be shown.")).toBeTruthy();
  });

  it("clears the message when another source shows", async () => {
    mocked.pdfViewShow.mockRejectedValueOnce({ kind: "not_found", message: "No such PDF", code: null });
    const { rerender } = render(<PdfPreview source={{ kind: "statement", id: "gone" }} />);
    await screen.findByText("This PDF can't be shown.");

    rerender(<PdfPreview source={{ kind: "statement", id: "s2" }} />);

    await waitFor(() => expect(screen.queryByText("This PDF can't be shown.")).toBeNull());
  });

  describe("moving along", () => {
    it("sends the new frame when the window is resized after the PDF is shown", async () => {
      render(<PdfPreview source={{ kind: "statement", id: "s1" }} />);
      await waitFor(() => expect(mocked.pdfViewMove).toHaveBeenCalled());
      mocked.pdfViewMove.mockClear();
      rect = { left: 10, top: 20, width: 500, height: 600 };

      fireEvent(window, new Event("resize"));

      expect(mocked.pdfViewMove).toHaveBeenCalledWith({ x: 10, y: 20, width: 500, height: 600 });
    });

    it("sends the new frame when something scrolls", async () => {
      render(<PdfPreview source={{ kind: "statement", id: "s1" }} />);
      await waitFor(() => expect(mocked.pdfViewMove).toHaveBeenCalled());
      mocked.pdfViewMove.mockClear();
      rect = { left: 10, top: -80, width: 300, height: 400 };

      fireEvent.scroll(document.body);

      expect(mocked.pdfViewMove).toHaveBeenCalledWith({ x: 10, y: -80, width: 300, height: 400 });
    });

    it("sends no move before the PDF is shown", async () => {
      const shown = deferred();
      mocked.pdfViewShow.mockReturnValue(shown.promise);
      render(<PdfPreview source={{ kind: "statement", id: "s1" }} />);

      fireEvent(window, new Event("resize"));
      expect(mocked.pdfViewMove).not.toHaveBeenCalled();

      shown.resolve();
      await waitFor(() => expect(mocked.pdfViewMove).toHaveBeenCalled());
    });

    it("sends no move after unmount", async () => {
      const { unmount } = render(<PdfPreview source={{ kind: "statement", id: "s1" }} />);
      await waitFor(() => expect(mocked.pdfViewMove).toHaveBeenCalled());
      mocked.pdfViewMove.mockClear();
      unmount();

      fireEvent(window, new Event("resize"));

      expect(mocked.pdfViewMove).not.toHaveBeenCalled();
    });

    it("ignores a rejected move", async () => {
      mocked.pdfViewMove.mockRejectedValue({ kind: "internal", message: "No view", code: null });
      render(<PdfPreview source={{ kind: "statement", id: "s1" }} />);
      await waitFor(() => expect(mocked.pdfViewMove).toHaveBeenCalled());

      fireEvent(window, new Event("resize"));

      expect(screen.queryByText("This PDF can't be shown.")).toBeNull();
    });
  });
});
