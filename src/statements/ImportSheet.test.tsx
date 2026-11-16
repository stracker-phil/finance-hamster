import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { Toasts } from "../components/Toasts";
import { useStore } from "../state/store";
import { PdfCoversProbe } from "../test/PdfCoversProbe";
import { importPreview } from "../test/statements";
import { ImportSheet } from "./ImportSheet";

const mocked = vi.mocked(api, true);
const preview = importPreview();

beforeEach(() => {
  vi.clearAllMocks();
  useStore.setState({ importPreview: preview, toasts: [], pdfCovers: [] });
  mocked.statementImportCommit.mockResolvedValue("Imported Nr. 010/2026");
  mocked.statementImportCancel.mockResolvedValue(undefined);
});

const renderSheet = (onImported?: () => void, shown = preview) =>
  render(
    <>
      <ImportSheet preview={shown} onImported={onImported} />
      <Toasts />
    </>,
  );

describe("ImportSheet", () => {
  it("shows what was read: file, summary, account, line count, balance, chain and target file", () => {
    renderSheet();

    expect(screen.getByRole("dialog", { name: "Import statement" })).toBeTruthy();
    expect(screen.getByText("statement-010-2026.pdf")).toBeTruthy();
    expect(screen.getByText("Kontoauszug Nr. 010/2026 · 4 pages · 1 Nov – 10 Nov 2026")).toBeTruthy();
    expect(screen.getByText("Private")).toBeTruthy();
    expect(screen.getByText("Read 42 lines")).toBeTruthy();
    expect(screen.getByText(/1,412.30 \+ 2,250.00 − 1,986.44 = 1,675.86/)).toBeTruthy();
    expect(screen.getByText("Follows statement Nr. 009/2026")).toBeTruthy();
    expect(screen.getByText("Opening balance matches the previous closing balance.")).toBeTruthy();
    expect(screen.getByText("Hamster keeps the PDF unchanged in its data folder.").getAttribute("title")).toBe("statements/2026/2026-11-10 Private.pdf");
    expect(screen.queryByText("statements/2026/2026-11-10 Private.pdf")).toBeNull();
  });

  it("shows the title and detail of a gap in the chain", () => {
    renderSheet(undefined, importPreview({ chain: "gap", chainTitle: "Statement Nr. 008 is missing", chainDetail: "Import it first to close the gap." }));

    expect(screen.getByText("Statement Nr. 008 is missing")).toBeTruthy();
    expect(screen.getByText("Import it first to close the gap.")).toBeTruthy();
  });

  it("hides the PDF preview while open", async () => {
    render(
      <>
        <PdfCoversProbe />
        <ImportSheet preview={preview} />
      </>,
    );

    await waitFor(() => expect(mocked.pdfViewCover).toHaveBeenLastCalledWith({ hidden: true, holes: [], clickThrough: false }));
  });

  describe("Cancel", () => {
    it("discards the read statement by its token and clears the preview", async () => {
      renderSheet();

      await userEvent.click(screen.getByRole("button", { name: "Cancel" }));

      expect(mocked.statementImportCancel).toHaveBeenCalledWith("tok-1");
      expect(mocked.statementImportCommit).not.toHaveBeenCalled();
      expect(useStore.getState().importPreview).toBeNull();
    });

    it("clears the preview on Escape too", async () => {
      renderSheet();

      await userEvent.keyboard("{Escape}");

      expect(mocked.statementImportCancel).toHaveBeenCalledWith("tok-1");
      expect(useStore.getState().importPreview).toBeNull();
    });

    it("still clears the preview when Rust cannot discard the statement", async () => {
      mocked.statementImportCancel.mockRejectedValue({ kind: "io", message: "Gone", code: null });
      renderSheet();

      await userEvent.click(screen.getByRole("button", { name: "Cancel" }));

      expect(useStore.getState().importPreview).toBeNull();
      expect(screen.queryByRole("alert")).toBeNull();
    });
  });

  describe("Import", () => {
    it("commits the statement by its token, shows the result in a toast and clears the preview", async () => {
      renderSheet();

      await userEvent.click(screen.getByRole("button", { name: "Import" }));

      expect((await screen.findByRole("status")).textContent).toContain("Imported Nr. 010/2026");
      expect(mocked.statementImportCommit).toHaveBeenCalledWith("tok-1");
      expect(useStore.getState().importPreview).toBeNull();
    });

    it("tells the caller once it is imported", async () => {
      const onImported = vi.fn();
      renderSheet(onImported);

      await userEvent.click(screen.getByRole("button", { name: "Import" }));

      await waitFor(() => expect(onImported).toHaveBeenCalled());
    });

    it("keeps the sheet open, shows the error and enables Import again when committing fails", async () => {
      mocked.statementImportCommit.mockRejectedValue({ kind: "invalid_input", message: "Statement already imported", code: null });
      const onImported = vi.fn();
      renderSheet(onImported);

      await userEvent.click(screen.getByRole("button", { name: "Import" }));

      expect((await screen.findByRole("alert")).textContent).toContain("Statement already imported");
      expect(useStore.getState().importPreview).toEqual(preview);
      expect(onImported).not.toHaveBeenCalled();
      expect(screen.getByRole("button", { name: "Import" })).toBeEnabled();
    });

    it("disables Import while the commit runs", async () => {
      mocked.statementImportCommit.mockReturnValue(new Promise(() => {}));
      renderSheet();

      await userEvent.click(screen.getByRole("button", { name: "Import" }));

      expect(screen.getByRole("button", { name: "Import" })).toBeDisabled();
    });
  });
});
