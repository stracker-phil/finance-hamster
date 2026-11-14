import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { Toasts } from "../components/Toasts";
import { useStore } from "../state/store";
import { addPreview, estimate } from "../test/documents";
import { renderWithQuery as render } from "../test/renderWithQuery";
import { AddDocumentSheet } from "./AddDocumentSheet";

const mocked = vi.mocked(api, true);
const preview = addPreview();

beforeEach(() => {
  vi.clearAllMocks();
  useStore.setState({ addPreview: preview, toasts: [] });
  mocked.documentEstimate.mockResolvedValue(estimate());
  mocked.documentAddCommit.mockResolvedValue("Added Demo Cloud Ltd");
});

const renderSheet = (shown = preview) =>
  render(
    <>
      <AddDocumentSheet preview={shown} />
      <Toasts />
    </>,
  );

const vendor = () => screen.getByRole("textbox", { name: "Vendor" });
const item = () => screen.getByRole("combobox", { name: "Item" });
const lineRadio = (name: string | RegExp) => screen.getByRole("radio", { name });
const addButton = () => screen.getByRole("button", { name: "Add Document" });

describe("AddDocumentSheet", () => {
  describe("what was read", () => {
    it("shows the file, the invoice and its amount", () => {
      renderSheet();

      expect(screen.getByRole("dialog", { name: "Add document" })).toBeTruthy();
      expect(screen.getByText("invoice-demo-1109.pdf")).toBeTruthy();
      expect(screen.getByText("DEM-2026-1109 · 9 Nov 2026")).toBeTruthy();
      expect(screen.getByText("USD 129.00")).toBeTruthy();
    });

    it("leaves out the invoice row when the PDF gave neither a number nor an amount", () => {
      renderSheet(addPreview({ invoice: null, invoiceAmount: null }));

      expect(screen.queryByText("Invoice")).toBeNull();
    });

    it("says nothing similar was added before", () => {
      renderSheet();

      expect(screen.getByText(/Not added before · no similar document found/)).toBeTruthy();
    });

    it("shows the near-duplicate warning instead", () => {
      renderSheet(addPreview({ nearDuplicate: "Same vendor and amount on 6 Nov 2026" }));

      expect(screen.getByText(/Same vendor and amount on 6 Nov 2026/)).toBeTruthy();
      expect(screen.queryByText(/Not added before/)).toBeNull();
    });
  });

  describe("vendor", () => {
    it("is prefilled and marked as read from the PDF", () => {
      renderSheet();

      expect((vendor() as HTMLInputElement).value).toBe("Demo Cloud Ltd");
      expect(screen.getByText("read from PDF")).toBeTruthy();
    });

    it("loses the read-from-PDF hint once edited", async () => {
      renderSheet();

      await userEvent.type(vendor(), " EU");

      expect(screen.queryByText("read from PDF")).toBeNull();
    });

    it("shows no hint when the vendor did not come from the PDF", () => {
      renderSheet(addPreview({ vendorFromPdf: false }));

      expect(screen.queryByText("read from PDF")).toBeNull();
    });

    it("is empty when unknown, and Add Document stays disabled until one is typed", async () => {
      renderSheet(addPreview({ vendor: null, vendorFromPdf: false }));
      expect(addButton()).toBeDisabled();

      await userEvent.type(vendor(), "   ");
      expect(addButton()).toBeDisabled();

      await userEvent.type(vendor(), "Demo Vendor");
      expect(addButton()).toBeEnabled();
    });
  });

  describe("item", () => {
    it("is prefilled with the reason for it", () => {
      renderSheet();

      expect((item() as HTMLSelectElement).value).toBe("kz-169");
      expect(screen.getByText("last used for Demo Cloud Ltd")).toBeTruthy();
    });

    it("loses the reason once another item is chosen", async () => {
      renderSheet();

      await userEvent.selectOptions(item(), "KZ 720 · Training");

      expect(screen.queryByText("last used for Demo Cloud Ltd")).toBeNull();
    });

    it("offers Choose later when no item is known", () => {
      renderSheet(addPreview({ itemKey: null, itemWhy: null }));

      expect((item() as HTMLSelectElement).value).toBe("");
      expect(screen.getByRole("option", { name: "Choose later" })).toBeTruthy();
    });
  });

  describe("Paid with", () => {
    it("offers every line with its detail and amount, and the first is chosen", () => {
      renderSheet();

      expect(lineRadio("Demo Cloud Ltd, Business · Tue, 10 Nov · USD 129.00, −€121.23")).toBeChecked();
      expect(lineRadio("Example Market, Business · Mon, 9 Nov · EUR 12.00, −€12.00")).not.toBeChecked();
      expect(lineRadio("Not imported yet, link later")).not.toBeChecked();
    });

    it("chooses Not imported yet when there is no line", () => {
      renderSheet(addPreview({ lines: [] }));

      expect(lineRadio("Not imported yet, link later")).toBeChecked();
    });

    it("moves the choice to the line clicked", async () => {
      renderSheet();

      await userEvent.click(lineRadio(/Example Market/));

      expect(lineRadio(/Example Market/)).toBeChecked();
      expect(lineRadio(/^Demo Cloud Ltd, /)).not.toBeChecked();
    });
  });

  describe("estimate", () => {
    it("asks for the preselected line and item, and shows the work share, its hint and what counts", async () => {
      renderSheet();

      expect(await screen.findByText("60 % · from the item: 40 % private")).toBeTruthy();
      expect(screen.getByText(/Counts as €72.74 \(€121.23 × 60 %\)\. Saved as a draft; finish it whenever you like\./)).toBeTruthy();
      expect(mocked.documentEstimate).toHaveBeenCalledWith("line-1", "kz-169");
    });

    it("shows the work share without a hint when there is none", async () => {
      mocked.documentEstimate.mockResolvedValue(estimate({ shareHint: null }));
      renderSheet();

      expect(await screen.findByText("60 %")).toBeTruthy();
    });

    it("shows the GWG text when the estimate has one", async () => {
      mocked.documentEstimate.mockResolvedValue(estimate({ gwg: "€121.23 is below the GWG limit (€1,000.00): counts fully in 2026." }));
      renderSheet();

      expect(await screen.findByText(/below the GWG limit/)).toBeTruthy();
    });

    it("asks again with no line and no item when both are cleared", async () => {
      renderSheet();
      await userEvent.selectOptions(item(), "Choose later");
      await userEvent.click(lineRadio("Not imported yet, link later"));

      await waitFor(() => expect(mocked.documentEstimate).toHaveBeenLastCalledWith(null, null));
    });

    it("shows only the draft note while there is no estimate", async () => {
      mocked.documentEstimate.mockResolvedValue(null);
      renderSheet();

      await waitFor(() => expect(mocked.documentEstimate).toHaveBeenCalled());
      expect(screen.getByText("Saved as a draft; finish it whenever you like.")).toBeTruthy();
      expect(screen.queryByText("Work share")).toBeNull();
    });
  });

  describe("Add Document", () => {
    it("commits the token, vendor, item and line, toasts the result and clears the preview", async () => {
      renderSheet();

      await userEvent.click(addButton());

      expect((await screen.findByRole("status")).textContent).toContain("Added Demo Cloud Ltd");
      expect(mocked.documentAddCommit).toHaveBeenCalledWith({ token: "add-1", vendor: "Demo Cloud Ltd", itemKey: "kz-169", lineId: "line-1" });
      expect(useStore.getState().addPreview).toBeNull();
    });

    it("commits the edited vendor, no item and the line chosen", async () => {
      renderSheet();
      await userEvent.clear(vendor());
      await userEvent.type(vendor(), "Demo Vendor");
      await userEvent.selectOptions(item(), "Choose later");
      await userEvent.click(lineRadio(/Example Market/));

      await userEvent.click(addButton());

      await waitFor(() => expect(mocked.documentAddCommit).toHaveBeenCalledWith({ token: "add-1", vendor: "Demo Vendor", itemKey: null, lineId: "line-2" }));
    });

    it("commits no line when Not imported yet is chosen", async () => {
      renderSheet();
      await userEvent.click(lineRadio("Not imported yet, link later"));

      await userEvent.click(addButton());

      await waitFor(() => expect(mocked.documentAddCommit).toHaveBeenCalledWith(expect.objectContaining({ lineId: null })));
    });

    it("keeps the sheet and shows the field error as an alert when the vendor is refused", async () => {
      mocked.documentAddCommit.mockRejectedValue({
        kind: "invalid_input",
        message: "Check the vendor",
        code: null,
        field: { path: "document.vendor", message: "Vendor is too long" },
      });
      renderSheet();

      await userEvent.click(addButton());

      expect((await screen.findByRole("alert")).textContent).toBe("Vendor is too long");
      expect(useStore.getState().addPreview).toEqual(preview);
      expect(addButton()).toBeEnabled();
    });

    it("shows the message of an error without a field", async () => {
      mocked.documentAddCommit.mockRejectedValue({ kind: "io", message: "Disk full", code: null });
      renderSheet();

      await userEvent.click(addButton());

      expect((await screen.findByRole("alert")).textContent).toBe("Disk full");
      expect(useStore.getState().addPreview).toEqual(preview);
    });

    it("hands the added label to the callback given with the preview", async () => {
      const onAdded = vi.fn();
      useStore.getState().setAddPreview(preview, onAdded);
      renderSheet();

      await userEvent.click(addButton());

      await waitFor(() => expect(onAdded).toHaveBeenCalledWith("Added Demo Cloud Ltd"));
    });

    it("does not call the callback when the commit is refused", async () => {
      const onAdded = vi.fn();
      useStore.getState().setAddPreview(preview, onAdded);
      mocked.documentAddCommit.mockRejectedValue({ kind: "io", message: "Disk full", code: null });
      renderSheet();

      await userEvent.click(addButton());

      await screen.findByRole("alert");
      expect(onAdded).not.toHaveBeenCalled();
    });

    it("is disabled while the commit runs", async () => {
      mocked.documentAddCommit.mockReturnValue(new Promise(() => {}));
      renderSheet();

      await userEvent.click(addButton());

      expect(addButton()).toBeDisabled();
    });
  });

  describe("Cancel", () => {
    it("discards the read invoice by its token and clears the preview", async () => {
      renderSheet();

      await userEvent.click(screen.getByRole("button", { name: "Cancel" }));

      expect(mocked.documentAddCancel).toHaveBeenCalledWith("add-1");
      expect(mocked.documentAddCommit).not.toHaveBeenCalled();
      expect(useStore.getState().addPreview).toBeNull();
    });

    it("never calls the callback given with the preview", async () => {
      const onAdded = vi.fn();
      useStore.getState().setAddPreview(preview, onAdded);
      renderSheet();

      await userEvent.click(screen.getByRole("button", { name: "Cancel" }));

      expect(onAdded).not.toHaveBeenCalled();
      expect(useStore.getState().addOnAdded).toBeNull();
    });

    it("does the same on Escape", async () => {
      renderSheet();

      await userEvent.keyboard("{Escape}");

      expect(mocked.documentAddCancel).toHaveBeenCalledWith("add-1");
      expect(useStore.getState().addPreview).toBeNull();
    });

    it("still clears the preview when Rust cannot discard the invoice", async () => {
      mocked.documentAddCancel.mockRejectedValue({ kind: "io", message: "Gone", code: null });
      renderSheet();

      await userEvent.click(screen.getByRole("button", { name: "Cancel" }));

      expect(useStore.getState().addPreview).toBeNull();
      expect(screen.queryByRole("alert")).toBeNull();
    });
  });
});
