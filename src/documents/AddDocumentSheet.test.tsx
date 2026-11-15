import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { Toasts } from "../components/Toasts";
import { useStore } from "../state/store";
import { addPreview, estimate } from "../test/documents";
import { payslipsYear } from "../test/payslips";
import { choose, popUpOptions, popUpValue, withMenus } from "../test/popup";
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
    withMenus(
      <>
        <AddDocumentSheet preview={shown} />
        <Toasts />
      </>,
    ),
  );

const vendor = () => screen.getByRole("textbox", { name: "Vendor" });
const chooseItem = (option: string) => choose(userEvent, "Item", option);
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

      expect(screen.queryAllByText("Invoice").filter((el) => el.tagName !== "BUTTON")).toEqual([]);
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

      expect(popUpValue("Item")).toBe("KZ 169 · Digital work equipment");
      expect(screen.getByText("last used for Demo Cloud Ltd")).toBeTruthy();
    });

    it("loses the reason once another item is chosen", async () => {
      renderSheet();

      await chooseItem("KZ 720 · Training");

      expect(popUpValue("Item")).toBe("KZ 720 · Training");
      expect(screen.queryByText("last used for Demo Cloud Ltd")).toBeNull();
    });

    it("keeps the reason when the current item is chosen again", async () => {
      renderSheet();

      await chooseItem("KZ 169 · Digital work equipment");

      expect(screen.getByText("last used for Demo Cloud Ltd")).toBeTruthy();
    });

    it("shows Choose later when no item is known", () => {
      renderSheet(addPreview({ itemKey: null, itemWhy: null }));

      expect(popUpValue("Item")).toBe("Choose later");
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
      expect(screen.getByText(/Counts as €72.74 \(€121.23 × 60 %\)\. It is saved as a draft; finish it whenever you like\./)).toBeTruthy();
      expect(mocked.documentEstimate).toHaveBeenCalledWith("line-1", "kz-169");
    });

    it("shows the work share without a hint when there is none", async () => {
      mocked.documentEstimate.mockResolvedValue(estimate({ shareHint: null }));
      renderSheet();

      expect(await screen.findByText("60 %")).toBeTruthy();
    });

    it("shows the GWG text as a plain hint without an icon when the estimate has one", async () => {
      mocked.documentEstimate.mockResolvedValue(estimate({ gwg: "€121.23 is below the GWG limit (€1,000.00): counts fully in 2026." }));
      renderSheet();

      const hint = await screen.findByText(/below the GWG limit/);
      expect(hint.querySelector("svg")).toBeNull();
    });

    it("asks again with no line and no item when both are cleared", async () => {
      renderSheet();
      await chooseItem("Choose later");
      await userEvent.click(lineRadio("Not imported yet, link later"));

      await waitFor(() => expect(mocked.documentEstimate).toHaveBeenLastCalledWith(null, null));
    });

    it("shows only the draft note while there is no estimate", async () => {
      mocked.documentEstimate.mockResolvedValue(null);
      renderSheet();

      await waitFor(() => expect(mocked.documentEstimate).toHaveBeenCalled());
      expect(screen.getByText("It is saved as a draft; finish it whenever you like.")).toBeTruthy();
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
      await chooseItem("Choose later");
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

  describe("Payslip", () => {
    const kindButton = (name: "Invoice" | "Payslip") => screen.getByRole("button", { name });
    const addPayslip = () => screen.getByRole("button", { name: "Add Payslip" });
    const toastMessages = () => useStore.getState().toasts.map((t) => t.message);
    const years = [
      { year: 2027, state: "open" as const, ruleset: "2027 v1" },
      { year: 2026, state: "open" as const, ruleset: "2026 v1" },
      { year: 2025, state: "closed" as const, ruleset: "2025 v1" },
    ];
    const showPayslip = async (guess: [number, number] | null = [2026, 7], year = payslipsYear()) => {
      mocked.payslipGuess.mockResolvedValue(guess);
      mocked.payslipsYear.mockResolvedValue(year);
      renderSheet();
      await userEvent.click(kindButton("Payslip"));
      await waitFor(() => expect(popUpValue("Month")).not.toBe(""));
      if (guess) await screen.findByText("from the PDF");
    };

    beforeEach(() => {
      useStore.setState({ route: { page: "today" }, back: [], forward: [] });
      mocked.listTaxYears.mockResolvedValue(years);
      mocked.payslipAddWaiting.mockResolvedValue({ id: "ps-new", label: "Add Payslip July 2026" });
    });

    it("starts on Invoice and shows the invoice fields", () => {
      renderSheet();

      expect(kindButton("Invoice").getAttribute("aria-pressed")).toBe("true");
      expect(kindButton("Payslip").getAttribute("aria-pressed")).toBe("false");
      expect(vendor()).toBeTruthy();
    });

    it("swaps the invoice fields for the payslip fields when Payslip is chosen", async () => {
      await showPayslip(null);

      expect(kindButton("Payslip").getAttribute("aria-pressed")).toBe("true");
      expect(screen.queryByRole("textbox", { name: "Vendor" })).toBeNull();
      expect(screen.queryByRole("button", { name: "Add Document" })).toBeNull();
      expect(screen.getByRole("button", { name: "Tax year" })).toBeTruthy();
      expect(screen.getByRole("button", { name: "Month" })).toBeTruthy();
      expect(addPayslip()).toBeTruthy();
    });

    it("brings the invoice fields back when Invoice is chosen again", async () => {
      await showPayslip(null);

      await userEvent.click(kindButton("Invoice"));

      expect(vendor()).toBeTruthy();
      expect(screen.queryByRole("button", { name: "Add Payslip" })).toBeNull();
    });

    it("offers the open tax years only", async () => {
      await showPayslip(null);
      await waitFor(() => expect(popUpValue("Tax year")).not.toBe(""));

      expect(await popUpOptions(userEvent, "Tax year")).toEqual(["2027", "2026"]);
    });

    it("offers the twelve months and labels one that already has a payslip", async () => {
      await showPayslip([2026, 3], payslipsYear({ filled: { 5: {} } }));

      const options = await popUpOptions(userEvent, "Month");
      expect(options).toHaveLength(12);
      expect(options[4]).toBe("May · already has one");
      expect(options[5]).toBe("June");
    });

    it("preselects the newest open tax year when the PDF gives no guess", async () => {
      mocked.listTaxYears.mockResolvedValue([
        { year: 2026, state: "open", ruleset: "2026 v1" },
        { year: 2025, state: "open", ruleset: "2025 v1" },
      ]);
      await showPayslip(null);

      await waitFor(() => expect(mocked.payslipsYear).toHaveBeenCalledWith(2026));
      expect(popUpValue("Tax year")).toBe("2026");
      expect(mocked.payslipsYear).not.toHaveBeenCalledWith(2025);
    });

    describe("the guess from the PDF", () => {
      it("preselects the guessed year and month and says so", async () => {
        await showPayslip([2026, 7]);

        expect(mocked.payslipGuess).toHaveBeenCalledWith("add-1");
        expect(popUpValue("Tax year")).toBe("2026");
        expect(popUpValue("Month")).toBe("July");
        expect(screen.getByText("from the PDF")).toBeTruthy();
      });

      it("shows no hint when the PDF gives no guess", async () => {
        await showPayslip(null);

        await waitFor(() => expect(mocked.payslipGuess).toHaveBeenCalled());
        expect(screen.queryByText("from the PDF")).toBeNull();
      });

      it("keeps the sheet usable when the guess cannot be read", async () => {
        mocked.payslipGuess.mockRejectedValue({ kind: "io", message: "Unreadable", code: null });
        mocked.payslipsYear.mockResolvedValue(payslipsYear());
        renderSheet();

        await userEvent.click(kindButton("Payslip"));

        await waitFor(() => expect(mocked.payslipGuess).toHaveBeenCalled());
        expect(screen.queryByRole("alert")).toBeNull();
        expect(screen.queryByText("from the PDF")).toBeNull();
        expect(addPayslip()).toBeTruthy();
      });
    });

    describe("a month that already has a payslip", () => {
      it("disables Add Payslip until another month is chosen", async () => {
        await showPayslip([2026, 7], payslipsYear({ filled: { 7: {} } }));
        expect(addPayslip()).toBeDisabled();

        await choose(userEvent, "Month", "August");
        await waitFor(() => expect(addPayslip()).toBeEnabled());

        await choose(userEvent, "Month", "July · already has one");
        expect(addPayslip()).toBeDisabled();
      });
    });

    describe("Add Payslip", () => {
      it("adds the waiting PDF as the month's payslip, closes the sheet, says so and opens the payslip", async () => {
        await showPayslip([2026, 7]);
        await waitFor(() => expect(addPayslip()).toBeEnabled());

        await userEvent.click(addPayslip());

        expect(mocked.payslipAddWaiting).toHaveBeenCalledWith("add-1", 2026, 7);
        await waitFor(() => expect(useStore.getState().addPreview).toBeNull());
        expect(useStore.getState().route).toEqual({ page: "payslip", payslipId: "ps-new", year: 2026 });
        expect(toastMessages()).toEqual(["Add Payslip July 2026"]);
        expect(useStore.getState().toasts[0].action?.label).toBe("Undo");
      });

      it("adds for the month and year chosen by hand", async () => {
        await showPayslip(null);
        await choose(userEvent, "Tax year", "2027");
        await choose(userEvent, "Month", "March");
        await waitFor(() => expect(addPayslip()).toBeEnabled());

        await userEvent.click(addPayslip());

        await waitFor(() => expect(mocked.payslipAddWaiting).toHaveBeenCalledWith("add-1", 2027, 3));
      });

      it("keeps the sheet, reports an error toast and enables the button again when refused", async () => {
        mocked.payslipAddWaiting.mockRejectedValue({ kind: "conflict", message: "July already has a payslip", code: null });
        await showPayslip([2026, 7]);
        await waitFor(() => expect(addPayslip()).toBeEnabled());

        await userEvent.click(addPayslip());

        await waitFor(() => expect(toastMessages()).toEqual(["July already has a payslip"]));
        expect(useStore.getState().addPreview).toEqual(preview);
        expect(useStore.getState().route).toEqual({ page: "today" });
        expect(addPayslip()).toBeEnabled();
      });

      it("is disabled while the add runs", async () => {
        mocked.payslipAddWaiting.mockReturnValue(new Promise(() => {}));
        await showPayslip([2026, 7]);
        await waitFor(() => expect(addPayslip()).toBeEnabled());

        await userEvent.click(addPayslip());

        expect(addPayslip()).toBeDisabled();
      });
    });

    it("discards the waiting PDF by its token on Cancel", async () => {
      await showPayslip(null);

      await userEvent.click(screen.getByRole("button", { name: "Cancel" }));

      expect(mocked.documentAddCancel).toHaveBeenCalledWith("add-1");
      expect(mocked.payslipAddWaiting).not.toHaveBeenCalled();
      expect(useStore.getState().addPreview).toBeNull();
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
