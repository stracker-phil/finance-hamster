import { fireEvent, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { useStore } from "../state/store";
import { depreciationView, documentView, estimate, lineChoice } from "../test/documents";
import { renderWithQuery } from "../test/renderWithQuery";
import { DocumentForm } from "./DocumentForm";

const mocked = vi.mocked(api, true);

const onExclude = vi.fn();
const renderForm = (patch: Parameters<typeof documentView>[0] = {}) => renderWithQuery(<DocumentForm doc={documentView(patch)} onExclude={onExclude} />);
const typeInto = async (label: string, text: string) => {
  const field = screen.getByLabelText(label);
  await userEvent.clear(field);
  await userEvent.type(field, text);
};
const unpaid = { paidFromLines: false, paid: null, paidText: null, lines: [] };

beforeEach(() => {
  vi.clearAllMocks();
  mocked.documentStep.mockResolvedValue("Finished Demo Cloud Ltd");
  useStore.setState({ toasts: [], pdfCovers: 0 });
  mocked.documentEdit.mockResolvedValue(null);
  mocked.documentLineOptions.mockResolvedValue([]);
});

describe("DocumentForm", () => {
  describe("status", () => {
    it("lists the missing facts", () => {
      renderForm({ missing: ["payment date", "vendor"] });

      expect(screen.getByText("Missing: payment date, vendor")).toBeTruthy();
    });

    it("says every required fact is set when nothing is missing", () => {
      renderForm({ missing: [] });

      expect(screen.getByText("Every required fact is set")).toBeTruthy();
    });
  });

  describe("payment date", () => {
    it("saves a changed date", () => {
      renderForm();

      fireEvent.change(screen.getByLabelText("Date"), { target: { value: "2026-11-12" } });

      expect(mocked.documentEdit).toHaveBeenCalledWith("doc-1", { field: "payDate", value: "2026-11-12" });
    });

    it("saves null when the date is cleared", () => {
      renderForm();

      fireEvent.change(screen.getByLabelText("Date"), { target: { value: "" } });

      expect(mocked.documentEdit).toHaveBeenCalledWith("doc-1", { field: "payDate", value: null });
    });

    it("names the tax year of the date", () => {
      renderForm({ year: 2026 });

      expect(screen.getByText("Tax year 2026")).toBeTruthy();
    });

    it("names no tax year while there is no date", () => {
      renderForm({ payDate: null, year: null });

      expect(screen.queryByText(/^Tax year/)).toBeNull();
    });
  });

  describe("bank lines", () => {
    it("lists the linked lines with their amount", () => {
      renderForm();

      expect(screen.getByText("Demo Cloud Ltd", { selector: ".line-picker__text span" })).toBeTruthy();
      expect(screen.getByText("Business · Tue, 10 Nov · USD 129.00")).toBeTruthy();
      expect(screen.getByText("−€121.23")).toBeTruthy();
    });

    it("unlinks a line", async () => {
      renderForm();

      await userEvent.click(screen.getByRole("button", { name: "Unlink Demo Cloud Ltd" }));

      expect(mocked.documentEdit).toHaveBeenCalledWith("doc-1", { field: "unlink", line: "line-1" });
    });

    it("offers Link a Bank Line… without lines and Link Another Payment… with one", () => {
      const { unmount } = renderForm({ lines: [] });
      expect(screen.getByRole("button", { name: "Link a Bank Line…" })).toBeTruthy();
      unmount();

      renderForm();
      expect(screen.getByRole("button", { name: "Link Another Payment…" })).toBeTruthy();
    });

    it("links the chosen line from the picker and closes it", async () => {
      mocked.documentLineOptions.mockResolvedValue([lineChoice({ id: "line-9", title: "Example Market" })]);
      renderForm({ lines: [] });
      await userEvent.click(screen.getByRole("button", { name: "Link a Bank Line…" }));

      await userEvent.click(await screen.findByRole("option", { name: /Example Market/ }));

      expect(mocked.documentEdit).toHaveBeenCalledWith("doc-1", { field: "link", line: "line-9" });
      expect(screen.queryByRole("dialog", { name: "Link a bank line" })).toBeNull();
    });
  });

  describe("paid", () => {
    it("shows the amount of the linked line, which cannot be typed", () => {
      renderForm();

      expect(screen.getByText("€121.23", { selector: ".doc-form__amount" })).toBeTruthy();
      expect(screen.getByText("from the bank line")).toBeTruthy();
      expect(screen.queryByLabelText("Paid")).toBeNull();
    });

    it("says lines when several are linked", () => {
      renderForm({ lines: [lineChoice(), lineChoice({ id: "line-2" })] });

      expect(screen.getByText("from the bank lines")).toBeTruthy();
    });

    it("saves the typed amount when the field loses focus", async () => {
      renderForm(unpaid);

      await userEvent.type(screen.getByLabelText("Paid"), "12.50");
      await userEvent.tab();

      expect(mocked.documentEdit).toHaveBeenCalledWith("doc-1", { field: "paid", value: "12.50" });
    });

    it("saves the typed amount on Enter", async () => {
      renderForm(unpaid);

      await userEvent.type(screen.getByLabelText("Paid"), "12.50{Enter}");

      expect(mocked.documentEdit).toHaveBeenCalledWith("doc-1", { field: "paid", value: "12.50" });
    });

    it("saves null when the amount is emptied", async () => {
      renderForm({ ...unpaid, paidText: "12.50" });

      await userEvent.clear(screen.getByLabelText("Paid"));
      await userEvent.tab();

      expect(mocked.documentEdit).toHaveBeenCalledWith("doc-1", { field: "paid", value: null });
    });

    it("saves nothing when the field is left unchanged", async () => {
      renderForm({ ...unpaid, paidText: "12.50" });

      await userEvent.click(screen.getByLabelText("Paid"));
      await userEvent.tab();

      expect(mocked.documentEdit).not.toHaveBeenCalled();
    });
  });

  describe("vendor", () => {
    it("saves the typed vendor when the field loses focus", async () => {
      renderForm();

      await typeInto("Vendor", "Example Market");
      await userEvent.tab();

      expect(mocked.documentEdit).toHaveBeenCalledWith("doc-1", { field: "vendor", name: "Example Market" });
    });

    it("suggests the known vendors", () => {
      renderForm();

      const options = Array.from((screen.getByLabelText<HTMLInputElement>("Vendor").list as HTMLDataListElement).options).map((o) => o.value);
      expect(options).toEqual(["Demo Cloud Ltd", "Example Market"]);
    });

    it("shows a field error under the vendor field", async () => {
      mocked.documentEdit.mockRejectedValue({ kind: "invalid_input", message: "document.vendor: Name is too long", code: null, field: { path: "document.vendor", message: "Name is too long" } });
      renderForm();

      await typeInto("Vendor", "Example Market");
      await userEvent.tab();

      expect((await screen.findByRole("alert")).textContent).toBe("Name is too long");
      expect(useStore.getState().toasts).toEqual([]);
    });

    it("clears the field error once a later save succeeds", async () => {
      mocked.documentEdit.mockRejectedValueOnce({ kind: "invalid_input", message: "x", code: null, field: { path: "document.vendor", message: "Name is too long" } });
      renderForm();
      await typeInto("Vendor", "Example Market");
      await userEvent.tab();
      await screen.findByRole("alert");

      await typeInto("Vendor", "Example");
      await userEvent.tab();

      await waitFor(() => expect(screen.queryByRole("alert")).toBeNull());
    });

    it("toasts an error that belongs to no field", async () => {
      mocked.documentEdit.mockRejectedValue({ kind: "io", message: "Disk full", code: null });
      renderForm();

      await typeInto("Vendor", "Example Market");
      await userEvent.tab();

      await waitFor(() => expect(useStore.getState().toasts.map((t) => [t.kind, t.message])).toEqual([["error", "Disk full"]]));
      expect(screen.queryByRole("alert")).toBeNull();
    });
  });

  describe("item", () => {
    it("is disabled and asks for the payment date first while there is none", () => {
      renderForm({ payDate: null, year: null, items: [], itemKey: null, estimate: null });

      expect(screen.getByLabelText("Item")).toBeDisabled();
      expect(screen.getByRole("option", { name: "Set the payment date first" })).toBeTruthy();
    });

    it("saves the chosen item", async () => {
      renderForm();

      await userEvent.selectOptions(screen.getByLabelText("Item"), "kz-720");

      expect(mocked.documentEdit).toHaveBeenCalledWith("doc-1", { field: "item", key: "kz-720" });
    });

    it("saves null when the empty choice is picked", async () => {
      renderForm();

      await userEvent.selectOptions(screen.getByLabelText("Item"), "Choose an item");

      expect(mocked.documentEdit).toHaveBeenCalledWith("doc-1", { field: "item", key: null });
    });
  });

  describe("ANV estimate", () => {
    it("shows the amount, its formula and the tax year it counts in", () => {
      renderForm();

      expect(screen.getByText("Counts in Tax year 2026")).toBeTruthy();
      expect(screen.getByText("€121.23 × 100 %")).toBeTruthy();
    });

    it("is absent until the amount paid is known", () => {
      renderForm({ estimate: null });

      expect(screen.queryByText(/Counts in/)).toBeNull();
      expect(screen.queryByRole("button", { name: "Partly private use…" })).toBeNull();
    });

    describe("GWG line", () => {
      const gwg = "€121.23 is below the GWG limit (€1,000.00): counts fully in 2026.";

      it("shows the text with a check icon when the purchase is not spread", () => {
        renderForm({ estimate: estimate({ gwg }) });

        expect(screen.getByText(gwg).querySelector("svg")).not.toBeNull();
      });

      it("shows the same text without the check icon when the purchase is spread", () => {
        renderForm({ estimate: estimate({ gwg }), depreciation: depreciationView() });

        expect(screen.getByText(gwg).querySelector("svg")).toBeNull();
      });
    });

    it("hides the work share behind Partly private use… while it is 100", async () => {
      renderForm();
      expect(screen.queryByLabelText("Work share")).toBeNull();

      await userEvent.click(screen.getByRole("button", { name: "Partly private use…" }));

      expect(screen.getByLabelText<HTMLInputElement>("Work share").value).toBe("100");
    });

    it("shows the work share and its hint at once when it is below 100", () => {
      renderForm({ workShare: 60, estimate: estimate() });

      expect(screen.getByLabelText<HTMLInputElement>("Work share").value).toBe("60");
      expect(screen.getByText("from the item: 40 % private")).toBeTruthy();
      expect(screen.queryByRole("button", { name: "Partly private use…" })).toBeNull();
    });

    it("saves the typed work share as a number", async () => {
      renderForm({ workShare: 60, estimate: estimate() });

      await typeInto("Work share", "75");
      await userEvent.tab();

      expect(mocked.documentEdit).toHaveBeenCalledWith("doc-1", { field: "workShare", value: 75 });
    });
  });

  describe("depreciation", () => {
    it("is absent when the purchase is not spread", () => {
      renderForm();

      expect(screen.queryByRole("heading", { name: "Depreciation" })).toBeNull();
      expect(screen.queryByLabelText("Useful life")).toBeNull();
    });

    it("shows the block with its slices when the purchase is spread", () => {
      renderForm({ depreciation: depreciationView() });

      expect(screen.getByRole("heading", { name: "Depreciation" })).toBeTruthy();
      expect(within(screen.getByRole("list", { name: "Slices" })).getAllByRole("listitem")).toHaveLength(4);
    });

    it("saves the chosen useful life", async () => {
      renderForm({ depreciation: depreciationView() });

      await userEvent.selectOptions(screen.getByLabelText("Useful life"), "5 years");

      expect(mocked.documentEdit).toHaveBeenCalledWith("doc-1", { field: "usefulLife", years: 5 });
    });

    it("saves null when the rules' years are chosen again", async () => {
      renderForm({ depreciation: depreciationView({ own: true, years: 5 }) });

      await userEvent.selectOptions(screen.getByLabelText("Useful life"), "As the rules say");

      expect(mocked.documentEdit).toHaveBeenCalledWith("doc-1", { field: "usefulLife", years: null });
    });

    it("shows a field error of the useful life", async () => {
      mocked.documentEdit.mockRejectedValue({ kind: "invalid_input", message: "x", code: null, field: { path: "document.usefulLife", message: "Useful life must be 1 to 15 years" } });
      renderForm({ depreciation: depreciationView() });

      await userEvent.selectOptions(screen.getByLabelText("Useful life"), "5 years");

      expect((await screen.findByRole("alert")).textContent).toBe("Useful life must be 1 to 15 years");
      expect(useStore.getState().toasts).toEqual([]);
    });

    it("is locked on a finished document", () => {
      renderForm({ state: "finished", depreciation: depreciationView() });

      expect(screen.getByLabelText("Useful life")).toBeDisabled();
    });
  });

  describe("note", () => {
    it("saves a changed note when it loses focus", async () => {
      renderForm();

      await userEvent.type(screen.getByRole("textbox", { name: "Note" }), "Bought for the training");
      await userEvent.tab();

      expect(mocked.documentEdit).toHaveBeenCalledWith("doc-1", { field: "note", text: "Bought for the training" });
    });

    it("saves nothing when the note is left unchanged", async () => {
      renderForm({ note: "Bought for the training" });

      await userEvent.click(screen.getByRole("textbox", { name: "Note" }));
      await userEvent.tab();

      expect(mocked.documentEdit).not.toHaveBeenCalled();
    });
  });

  describe("invoice amount", () => {
    it("is hidden until More: invoice amount is opened, and hides again", async () => {
      renderForm();
      const toggle = screen.getByRole("button", { name: "More: invoice amount" });
      expect(toggle.getAttribute("aria-expanded")).toBe("false");
      expect(screen.queryByLabelText("Invoice amount")).toBeNull();

      await userEvent.click(toggle);
      expect(toggle.getAttribute("aria-expanded")).toBe("true");
      expect(screen.getByLabelText<HTMLInputElement>("Currency").value).toBe("EUR");

      await userEvent.click(toggle);
      expect(screen.queryByLabelText("Invoice amount")).toBeNull();
    });

    it("is open from the start when the document has one", () => {
      renderForm({ invoiceAmount: "129.00", invoiceCurrency: "USD" });

      expect(screen.getByLabelText<HTMLInputElement>("Invoice amount").value).toBe("129.00");
      expect(screen.getByLabelText<HTMLInputElement>("Currency").value).toBe("USD");
    });

    it("saves the typed amount with the current currency", async () => {
      renderForm();
      await userEvent.click(screen.getByRole("button", { name: "More: invoice amount" }));

      await userEvent.type(screen.getByLabelText("Invoice amount"), "129.00");
      await userEvent.tab();

      expect(mocked.documentEdit).toHaveBeenCalledWith("doc-1", { field: "invoice", amount: "129.00", currency: "EUR" });
    });

    it("saves a changed currency with the current amount", async () => {
      renderForm({ invoiceAmount: "129.00", invoiceCurrency: "EUR" });

      await typeInto("Currency", "USD");
      await userEvent.tab();

      expect(mocked.documentEdit).toHaveBeenCalledWith("doc-1", { field: "invoice", amount: "129.00", currency: "USD" });
    });
  });

  describe("Mark Finished", () => {
    it("finishes the draft and toasts the result", async () => {
      mocked.documentStep.mockResolvedValue("Finished Demo Cloud Ltd");
      renderForm();

      await userEvent.click(screen.getByRole("button", { name: "Mark Finished" }));

      expect(mocked.documentStep).toHaveBeenCalledWith("doc-1", { kind: "finish" });
      await waitFor(() => expect(useStore.getState().toasts.map((t) => [t.kind, t.message])).toEqual([["success", "Finished Demo Cloud Ltd"]]));
    });

    it("is disabled and names the missing facts while some are missing", () => {
      renderForm({ missing: ["payment date", "vendor"] });

      const button = screen.getByRole("button", { name: "Mark Finished" });
      expect(button).toBeDisabled();
      expect(button.getAttribute("title")).toBe("Set the payment date, vendor first");
    });

    it.each(["finished", "excluded"] as const)("is absent when the document is %s", (state) => {
      renderForm({ state });

      expect(screen.queryByRole("button", { name: "Mark Finished" })).toBeNull();
    });
  });

  describe("Exclude Document…", () => {
    it("asks to exclude when clicked", async () => {
      renderForm();

      await userEvent.click(screen.getByRole("button", { name: "Exclude Document…" }));

      expect(onExclude).toHaveBeenCalled();
    });

    it("stays enabled for a finished document, whose fields are locked", async () => {
      renderForm({ state: "finished" });
      const link = screen.getByRole("button", { name: "Exclude Document…" });
      expect(link).toBeEnabled();
      expect(screen.getByLabelText("Vendor")).toBeDisabled();

      await userEvent.click(link);

      expect(onExclude).toHaveBeenCalled();
    });

    it("is absent for an excluded document", () => {
      renderForm({ state: "excluded" });

      expect(screen.queryByRole("button", { name: "Exclude Document…" })).toBeNull();
    });
  });

  describe("a document that is not a draft", () => {
    it.each(["finished", "excluded"] as const)("disables every field when it is %s", (state) => {
      renderForm({ state });

      expect(screen.getByRole("group")).toBeDisabled();
      expect(screen.getByLabelText("Vendor")).toBeDisabled();
      expect(screen.getByLabelText("Item")).toBeDisabled();
      expect(screen.getByRole("button", { name: "Unlink Demo Cloud Ltd" })).toBeDisabled();
      expect(screen.getByRole("textbox", { name: "Note" })).toBeDisabled();
    });
  });
});
