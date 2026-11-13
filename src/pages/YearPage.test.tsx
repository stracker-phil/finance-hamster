import { screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { HOME } from "../state/route";
import { useStore } from "../state/store";
import { addPreview } from "../test/documents";
import { renderWithQuery } from "../test/renderWithQuery";
import { attentionView, kennzahlView, monthView, questionView, yearPage, yearRow } from "../test/year";
import { YearPage } from "./YearPage";

const mocked = vi.mocked(api, true);

const show = async (patch: Parameters<typeof yearPage>[0] = {}) => {
  mocked.yearPage.mockResolvedValue(yearPage(patch));
  renderWithQuery(<YearPage year={2026} />);
  await screen.findByText("Total to enter for 2026 · ruleset 2026 v1");
};

beforeEach(() => {
  vi.clearAllMocks();
  useStore.setState({ route: HOME, back: [], forward: [], filter: "", toasts: [], addPreview: null, pdfCovers: 0 });
});

describe("YearPage", () => {
  describe("hero", () => {
    it("shows the total, the ruleset and the progress", async () => {
      await show();

      expect(screen.getByText("€1,228.40")).toBeTruthy();
      expect(screen.getByText("Draft · 1 invoice missing")).toBeTruthy();
    });

    it("shows the Pauschale note only when the year has one", async () => {
      await show({ pauschaleNote: "Below the Werbungskostenpauschale of €132" });

      expect(screen.getByText("Below the Werbungskostenpauschale of €132")).toBeTruthy();
    });

    it("shows no Pauschale note otherwise", async () => {
      await show();

      expect(screen.queryByText(/Pauschale/)).toBeNull();
    });

    it("says there is no tax year when its page cannot be read", async () => {
      mocked.yearPage.mockRejectedValue({ kind: "not_found", message: "No tax year", code: null });
      renderWithQuery(<YearPage year={2026} />);

      expect(await screen.findByText("No tax year 2026")).toBeTruthy();
      expect(useStore.getState().toasts).toEqual([]);
    });

    it("asks for the page with the header filter as the query", async () => {
      useStore.setState({ filter: "books" });

      await show();

      expect(mocked.yearPage).toHaveBeenCalledWith(2026, "books");
    });
  });

  describe("questions", () => {
    it("shows each question with its help, unit and the answer prefilled", async () => {
      await show({ questions: [questionView({ value: "42", answered: true })] });

      const section = screen.getByRole("region", { name: "Questions for 2026" });
      expect(within(section).getByText("Count whole days")).toBeTruthy();
      expect(within(section).getByText("days")).toBeTruthy();
      expect(within(section).getByRole<HTMLInputElement>("textbox", { name: "Days worked from home" }).value).toBe("42");
    });

    it("hides the questions of a closed year", async () => {
      await show({ state: "closed", questions: [questionView()] });

      expect(screen.queryByRole("region", { name: "Questions for 2026" })).toBeNull();
    });

    it("saves the typed answer on blur", async () => {
      await show({ questions: [questionView()] });

      await userEvent.type(screen.getByRole("textbox", { name: "Days worked from home" }), "42");
      await userEvent.tab();

      expect(mocked.yearAnswer).toHaveBeenCalledWith(2026, "commute", "42");
    });

    it("saves the typed answer on Enter", async () => {
      await show({ questions: [questionView()] });

      await userEvent.type(screen.getByRole("textbox", { name: "Days worked from home" }), "17{Enter}");

      expect(mocked.yearAnswer).toHaveBeenCalledWith(2026, "commute", "17");
    });

    it("saves nothing when the answer is unchanged", async () => {
      await show({ questions: [questionView({ value: "42", answered: true })] });

      await userEvent.click(screen.getByRole("textbox", { name: "Days worked from home" }));
      await userEvent.tab();

      expect(mocked.yearAnswer).not.toHaveBeenCalled();
    });

    it("reports an answer that cannot be saved as an error toast", async () => {
      mocked.yearAnswer.mockRejectedValue({ kind: "invalid_input", message: "Enter a number", code: null });
      await show({ questions: [questionView()] });

      await userEvent.type(screen.getByRole("textbox", { name: "Days worked from home" }), "x{Enter}");

      await vi.waitFor(() => expect(useStore.getState().toasts.map((t) => t.message)).toEqual(["Enter a number"]));
    });
  });

  describe("needs attention", () => {
    it("shows no section when nothing needs attention", async () => {
      await show();

      expect(screen.queryByRole("region", { name: "Needs attention" })).toBeNull();
    });

    it.each([
      ["draft", "Open Draft"],
      ["undated", "Open"],
    ] as const)("opens the document of a %s entry with the year from its %s button", async (kind, name) => {
      await show({ attention: [attentionView({ kind, documentId: "doc-2" })] });

      await userEvent.click(within(screen.getByRole("region", { name: "Needs attention" })).getByRole("button", { name }));

      expect(useStore.getState().route).toEqual({ page: "document", documentId: "doc-2", year: 2026 });
    });

    describe("with more than five entries", () => {
      const seven = () => Array.from({ length: 7 }, (_, i) => attentionView({ title: `Example Receipt ${i + 1}`, documentId: `doc-${i + 1}` }));

      it("shows the first five and a Show all link with the total", async () => {
        await show({ attention: seven() });

        const section = within(screen.getByRole("region", { name: "Needs attention" }));
        expect(section.getByText(/Example Receipt 5/)).toBeTruthy();
        expect(section.queryByText(/Example Receipt 6/)).toBeNull();
        expect(section.getByRole("button", { name: "Show all 7" })).toBeTruthy();
      });

      it("shows every entry after Show all, and the first five again after Show fewer", async () => {
        await show({ attention: seven() });

        await userEvent.click(screen.getByRole("button", { name: "Show all 7" }));
        expect(screen.getByText(/Example Receipt 7/)).toBeTruthy();

        await userEvent.click(screen.getByRole("button", { name: "Show fewer" }));
        expect(screen.queryByText(/Example Receipt 6/)).toBeNull();
        expect(screen.getByRole("button", { name: "Show all 7" })).toBeTruthy();
      });

      it("offers no link for exactly five entries", async () => {
        await show({ attention: seven().slice(0, 5) });

        expect(screen.getByText(/Example Receipt 5/)).toBeTruthy();
        expect(screen.queryByRole("button", { name: /Show all/ })).toBeNull();
      });
    });

    it("starts the add flow from a missing invoice's Attach PDF button", async () => {
      await show({ attention: [attentionView({ kind: "missing_invoice", title: "Example Hosting", detail: "No invoice yet", documentId: null })] });

      await userEvent.click(screen.getByRole("button", { name: "Attach PDF…" }));

      expect(mocked.documentAddChoose).toHaveBeenCalled();
    });
  });

  describe("by Kennzahl", () => {
    it("is the default view", async () => {
      await show();

      expect(screen.getByRole("button", { name: "By Kennzahl" }).getAttribute("aria-pressed")).toBe("true");
      expect(screen.getByRole("region", { name: "KZ 720 Fachliteratur" })).toBeTruthy();
    });

    it("copies a Kennzahl's plain number and says so", async () => {
      await show();

      await userEvent.click(screen.getByRole("button", { name: "Copy 1228,40 for KZ 720" }));

      expect(navigator.clipboard.writeText).toHaveBeenCalledWith("1228,40");
      await vi.waitFor(() => expect(useStore.getState().toasts.map((t) => t.message)).toEqual(["Copied 1228,40 for KZ 720"]));
    });

    it("reports a failed copy as an error toast", async () => {
      vi.mocked(navigator.clipboard.writeText).mockRejectedValueOnce({ kind: "io", message: "Clipboard unavailable", code: null });
      await show();

      await userEvent.click(screen.getByRole("button", { name: "Copy 1228,40 for KZ 720" }));

      await vi.waitFor(() => expect(useStore.getState().toasts.map((t) => t.message)).toEqual(["Clipboard unavailable"]));
    });

    it("shows a row's title, Draft pill and detail", async () => {
      await show({ kennzahlen: [kennzahlView({ rows: [yearRow({ draft: true })] })] });

      const row = screen.getByRole("button", { name: /Demo Books Ltd/ });
      expect(within(row).getByText("Draft")).toBeTruthy();
      expect(within(row).getByText("10 Nov · 60 % of €22.48")).toBeTruthy();
    });

    it("names the year a row is carried from", async () => {
      await show({ kennzahlen: [kennzahlView({ rows: [yearRow({ carriedFrom: 2025 })] })] });

      expect(within(screen.getByRole("button", { name: /Demo Books Ltd/ })).getByText("From 2025")).toBeTruthy();
    });

    it("shows no From pill on a row that is not carried", async () => {
      await show();

      expect(within(screen.getByRole("button", { name: /Demo Books Ltd/ })).queryByText(/^From /)).toBeNull();
    });

    it("shows row amounts only when a Kennzahl has several rows", async () => {
      await show({
        kennzahlen: [
          kennzahlView({ rows: [yearRow({ id: "d1", title: "Demo Books Ltd", amount: "€13.49" })] }),
          kennzahlView({ key: "169", kz: "169", name: "Digital work equipment", copy: "100,00", rows: [yearRow({ id: "d2", title: "Sample Laptop", amount: "€60.00" }), yearRow({ id: "d3", title: "Sample Monitor", amount: "€40.00" })] }),
        ],
      });

      expect(within(screen.getByRole("region", { name: "KZ 720 Fachliteratur" })).queryByText("€13.49")).toBeNull();
      const multi = within(screen.getByRole("region", { name: "KZ 169 Digital work equipment" }));
      expect(multi.getByText("€60.00")).toBeTruthy();
      expect(multi.getByText("€40.00")).toBeTruthy();
    });

    it("shows a Kennzahl's note", async () => {
      await show({ kennzahlen: [kennzahlView({ note: "Estimated from 3 documents" })] });

      expect(screen.getByText("Estimated from 3 documents")).toBeTruthy();
    });

    it("opens the document of a document row with the year", async () => {
      await show();

      await userEvent.click(screen.getByRole("button", { name: /Demo Books Ltd/ }));

      expect(useStore.getState().route).toEqual({ page: "document", documentId: "doc-1", year: 2026 });
    });

    it("keeps a bank line row from being opened", async () => {
      await show({ kennzahlen: [kennzahlView({ rows: [yearRow({ id: "line-1", kind: "line", title: "Example Union dues" })] })] });

      expect(screen.getByRole<HTMLButtonElement>("button", { name: /Example Union dues/ }).disabled).toBe(true);
    });
  });

  describe("adding to a Kennzahl", () => {
    const amountRow = (patch: Parameters<typeof yearRow>[0] = {}) => yearRow({ id: "e1", kind: "amount", title: "Example Desk Lamp", ...patch });
    const tripRow = () => yearRow({ id: "t1", kind: "trip", title: "Linz · client workshop" });
    const toastMessages = () => useStore.getState().toasts.map((t) => t.message);
    const group = (name: string) => within(screen.getByRole("group", { name }));

    beforeEach(() => {
      mocked.documentAddChoose.mockResolvedValue(null);
      mocked.yearEvidenceChoose.mockResolvedValue(null);
      mocked.yearEntryAdd.mockResolvedValue("Add entry");
      mocked.yearEntryDelete.mockResolvedValue("Delete entry");
    });

    describe("buttons in the Kennzahl head", () => {
      it.each([
        ["document", "+ Document"],
        ["amount", "+ Amount"],
        ["trip", "+ Trip"],
      ] as const)("shows %s as %s in an open year", async (add, name) => {
        await show({ kennzahlen: [kennzahlView({ add })] });

        expect(screen.getByRole("button", { name })).toBeTruthy();
      });

      it("shows no add button when the Kennzahl takes nothing to add", async () => {
        await show();

        expect(screen.queryByRole("button", { name: /^\+ / })).toBeNull();
      });

      it("shows no add button in a closed year", async () => {
        await show({ state: "closed", kennzahlen: [kennzahlView({ add: "amount" })] });

        expect(screen.queryByRole("button", { name: /^\+ / })).toBeNull();
      });

      it("shows Not counted instead of the copy button and no add button for a reported item", async () => {
        await show({ kennzahlen: [kennzahlView({ add: "amount", reported: true })] });

        expect(screen.getByText("Not counted")).toBeTruthy();
        expect(screen.queryByRole("button", { name: /^Copy / })).toBeNull();
        expect(screen.queryByRole("button", { name: /^\+ / })).toBeNull();
      });
    });

    describe("document", () => {
      it("opens the add sheet with the Kennzahl's item preset", async () => {
        const preview = addPreview();
        mocked.documentAddChoose.mockResolvedValue(preview);
        await show({ kennzahlen: [kennzahlView({ key: "720", add: "document" })] });

        await userEvent.click(screen.getByRole("button", { name: "+ Document" }));

        await vi.waitFor(() => expect(useStore.getState().addPreview?.itemKey).toBe("720"));
      });

      it("opens nothing when no PDF is chosen", async () => {
        await show({ kennzahlen: [kennzahlView({ add: "document" })] });

        await userEvent.click(screen.getByRole("button", { name: "+ Document" }));

        await vi.waitFor(() => expect(mocked.documentAddChoose).toHaveBeenCalled());
        expect(useStore.getState().addPreview).toBeNull();
      });
    });

    describe("trip", () => {
      const showTrip = () => show({ kennzahlen: [kennzahlView({ key: "trips", kz: "721", name: "Trips", add: "trip", rate: "€0.50 per km" })] });
      const openTrip = async () => {
        await showTrip();
        await userEvent.click(screen.getByRole("button", { name: "+ Trip" }));
      };
      const fillTrip = async () => {
        const fields = group("Add a trip to KZ 721");
        const date = fields.getByLabelText("Date");
        await userEvent.clear(date);
        await userEvent.type(date, "2026-11-05");
        await userEvent.type(fields.getByRole("textbox", { name: "Destination and purpose" }), "Linz · client workshop");
        await userEvent.type(fields.getByRole("textbox", { name: "Kilometres" }), "312");
      };

      it("opens an inline group with date, destination, kilometres and the rate", async () => {
        await openTrip();

        const fields = group("Add a trip to KZ 721");
        expect(fields.getByLabelText("Date")).toBeTruthy();
        expect(fields.getByRole("textbox", { name: "Destination and purpose" })).toBeTruthy();
        expect(fields.getByRole("textbox", { name: "Kilometres" })).toBeTruthy();
        expect(fields.getByText("€0.50 per km")).toBeTruthy();
      });

      it("closes the group when + Trip is clicked again", async () => {
        await openTrip();

        await userEvent.click(screen.getByRole("button", { name: "+ Trip" }));

        expect(screen.queryByRole("group", { name: "Add a trip to KZ 721" })).toBeNull();
      });

      it("sends the trip, closes the row and shows the label with Undo", async () => {
        mocked.yearEntryAdd.mockResolvedValue("Add trip Linz");
        await openTrip();
        await fillTrip();

        await userEvent.click(group("Add a trip to KZ 721").getByRole("button", { name: "Add" }));

        expect(mocked.yearEntryAdd).toHaveBeenCalledWith(2026, { kind: "trip", itemKey: "trips", date: "2026-11-05", km: "312", text: "Linz · client workshop" });
        await vi.waitFor(() => expect(screen.queryByRole("group", { name: "Add a trip to KZ 721" })).toBeNull());
        expect(toastMessages()).toEqual(["Add trip Linz"]);
        expect(useStore.getState().toasts[0].action?.label).toBe("Undo");
      });

      it("adds on Enter", async () => {
        await openTrip();
        await fillTrip();

        await userEvent.type(group("Add a trip to KZ 721").getByRole("textbox", { name: "Kilometres" }), "{Enter}");

        expect(mocked.yearEntryAdd).toHaveBeenCalledWith(2026, expect.objectContaining({ kind: "trip", km: "312" }));
      });

      it("closes on Escape without adding", async () => {
        await openTrip();

        await userEvent.type(group("Add a trip to KZ 721").getByRole("textbox", { name: "Kilometres" }), "{Escape}");

        expect(screen.queryByRole("group", { name: "Add a trip to KZ 721" })).toBeNull();
        expect(mocked.yearEntryAdd).not.toHaveBeenCalled();
      });

      it("shows only the field's message when the rejection names a field", async () => {
        mocked.yearEntryAdd.mockRejectedValue({ kind: "invalid_input", message: "entry.km: Whole kilometres", code: null, field: { path: "entry.km", message: "Whole kilometres" } });
        await openTrip();

        await userEvent.click(group("Add a trip to KZ 721").getByRole("button", { name: "Add" }));

        expect((await screen.findByRole("alert")).textContent).toBe("Whole kilometres");
      });

      it("shows the error under the row and keeps it open when the add is rejected", async () => {
        mocked.yearEntryAdd.mockRejectedValue({ kind: "invalid_input", message: "Enter the kilometres", code: null });
        await openTrip();

        await userEvent.click(group("Add a trip to KZ 721").getByRole("button", { name: "Add" }));

        expect((await screen.findByRole("alert")).textContent).toBe("Enter the kilometres");
        expect(screen.getByRole("group", { name: "Add a trip to KZ 721" })).toBeTruthy();
        expect(toastMessages()).toEqual([]);
      });
    });

    describe("amount", () => {
      const showAmount = async () => {
        await show({ kennzahlen: [kennzahlView({ key: "other", kz: "722", name: "Other", add: "amount" })] });
        await userEvent.click(screen.getByRole("button", { name: "+ Amount" }));
      };
      const name = "Add an amount to KZ 722";

      it("opens an inline group with note, amount and a PDF button", async () => {
        await showAmount();

        const fields = group(name);
        expect(fields.getByRole("textbox", { name: "Note" })).toBeTruthy();
        expect(fields.getByRole("textbox", { name: "Amount" })).toBeTruthy();
        expect(fields.getByRole("button", { name: "PDF…" })).toBeTruthy();
      });

      it("sends the amount without evidence when no PDF is chosen", async () => {
        await showAmount();
        await userEvent.type(group(name).getByRole("textbox", { name: "Note" }), "Desk lamp");
        await userEvent.type(group(name).getByRole("textbox", { name: "Amount" }), "29,90");

        await userEvent.click(group(name).getByRole("button", { name: "Add" }));

        expect(mocked.yearEntryAdd).toHaveBeenCalledWith(2026, expect.objectContaining({ kind: "amount", itemKey: "other", amount: "29,90", text: "Desk lamp", evidence: null }));
      });

      it("shows the chosen PDF's file name on the button and sends its path as evidence", async () => {
        mocked.yearEvidenceChoose.mockResolvedValue("/Users/example/Receipts/lamp-receipt.pdf");
        await showAmount();

        await userEvent.click(group(name).getByRole("button", { name: "PDF…" }));

        expect(await group(name).findByRole("button", { name: "lamp-receipt.pdf" })).toBeTruthy();
        await userEvent.type(group(name).getByRole("textbox", { name: "Amount" }), "29,90");
        await userEvent.click(group(name).getByRole("button", { name: "Add" }));
        expect(mocked.yearEntryAdd).toHaveBeenCalledWith(2026, expect.objectContaining({ kind: "amount", evidence: "/Users/example/Receipts/lamp-receipt.pdf" }));
      });

      it("keeps PDF… on the button when the file dialog is cancelled", async () => {
        await showAmount();

        await userEvent.click(group(name).getByRole("button", { name: "PDF…" }));

        await vi.waitFor(() => expect(mocked.yearEvidenceChoose).toHaveBeenCalled());
        expect(group(name).getByRole("button", { name: "PDF…" })).toBeTruthy();
      });
    });

    describe("entry rows", () => {
      it.each([
        ["amount", amountRow(), "Delete Example Desk Lamp"],
        ["trip", tripRow(), "Delete Linz · client workshop"],
      ] as const)("deletes a %s row with its Delete button", async (_kind, row, label) => {
        mocked.yearEntryDelete.mockResolvedValue("Delete entry");
        await show({ kennzahlen: [kennzahlView({ rows: [row] })] });

        await userEvent.click(screen.getByRole("button", { name: label }));

        expect(mocked.yearEntryDelete).toHaveBeenCalledWith(2026, row.id);
        await vi.waitFor(() => expect(toastMessages()).toEqual(["Delete entry"]));
      });

      it("reports a rejected delete as an error toast", async () => {
        mocked.yearEntryDelete.mockRejectedValue({ kind: "conflict", message: "Entry is locked", code: null });
        await show({ kennzahlen: [kennzahlView({ rows: [amountRow()] })] });

        await userEvent.click(screen.getByRole("button", { name: "Delete Example Desk Lamp" }));

        await vi.waitFor(() => expect(toastMessages()).toEqual(["Entry is locked"]));
      });

      it("offers neither Delete nor Attach PDF in a closed year", async () => {
        await show({ state: "closed", kennzahlen: [kennzahlView({ rows: [amountRow({ attach: true }), tripRow()] })] });

        expect(screen.queryByRole("button", { name: /^Delete / })).toBeNull();
        expect(screen.queryByRole("button", { name: "Attach PDF…" })).toBeNull();
      });

      it("offers no Delete for a document row", async () => {
        await show();

        expect(screen.queryByRole("button", { name: /^Delete / })).toBeNull();
      });

      it("attaches the chosen PDF to an amount row that lacks one", async () => {
        mocked.yearEvidenceChoose.mockResolvedValue("/Users/example/Receipts/lamp-receipt.pdf");
        await show({ kennzahlen: [kennzahlView({ rows: [amountRow({ attach: true })] })] });

        await userEvent.click(screen.getByRole("button", { name: "Attach PDF…" }));

        await vi.waitFor(() => expect(mocked.yearEntryAttach).toHaveBeenCalledWith(2026, "e1", "/Users/example/Receipts/lamp-receipt.pdf"));
      });

      it("attaches nothing when the file dialog is cancelled", async () => {
        await show({ kennzahlen: [kennzahlView({ rows: [amountRow({ attach: true })] })] });

        await userEvent.click(screen.getByRole("button", { name: "Attach PDF…" }));

        await vi.waitFor(() => expect(mocked.yearEvidenceChoose).toHaveBeenCalled());
        expect(mocked.yearEntryAttach).not.toHaveBeenCalled();
      });

      it("shows no Attach PDF button on an amount row that has its evidence", async () => {
        await show({ kennzahlen: [kennzahlView({ rows: [amountRow()] })] });

        expect(screen.queryByRole("button", { name: "Attach PDF…" })).toBeNull();
      });

      it("opens the evidence of an amount row that has one", async () => {
        await show({ kennzahlen: [kennzahlView({ rows: [amountRow()] })] });

        await userEvent.click(screen.getByRole("button", { name: /^Example Desk Lamp/ }));

        expect(mocked.yearEvidenceOpen).toHaveBeenCalledWith(2026, "e1");
      });

      it("keeps an amount row without evidence from being opened", async () => {
        await show({ kennzahlen: [kennzahlView({ rows: [amountRow({ attach: true })] })] });

        expect(screen.getByRole<HTMLButtonElement>("button", { name: /^Example Desk Lamp/ }).disabled).toBe(true);
      });

      it("keeps a trip row from being opened", async () => {
        await show({ kennzahlen: [kennzahlView({ rows: [tripRow()] })] });

        expect(screen.getByRole<HTMLButtonElement>("button", { name: /^Linz · client workshop/ }).disabled).toBe(true);
      });
    });

    describe("needs attention", () => {
      it("attaches a PDF to the entry of an attention item that names one", async () => {
        mocked.yearEvidenceChoose.mockResolvedValue("/Users/example/Receipts/lamp-receipt.pdf");
        await show({ attention: [attentionView({ kind: "missing_invoice", title: "Example Desk Lamp", documentId: null, entryId: "e1" })] });

        await userEvent.click(screen.getByRole("button", { name: "Attach PDF…" }));

        await vi.waitFor(() => expect(mocked.yearEntryAttach).toHaveBeenCalledWith(2026, "e1", "/Users/example/Receipts/lamp-receipt.pdf"));
        expect(mocked.documentAddChoose).not.toHaveBeenCalled();
      });
    });
  });

  describe("by month", () => {
    const byMonth = () => userEvent.click(screen.getByRole("button", { name: "By month" }));

    it("shows each month with its total and rows in place of the Kennzahlen", async () => {
      await show({ months: [monthView({ label: "November", total: "€13.49" }), monthView({ label: "Whole year", total: "€100.00", rows: [yearRow({ id: "d2", title: "Sample Laptop", amount: "€100.00" })] })] });

      await byMonth();

      expect(screen.getByRole("button", { name: "By month" }).getAttribute("aria-pressed")).toBe("true");
      expect(screen.queryByRole("region", { name: "KZ 720 Fachliteratur" })).toBeNull();
      const november = within(screen.getByRole("region", { name: "November" }));
      expect(november.getAllByText("€13.49")).toHaveLength(2);
      expect(november.getByRole("button", { name: /Demo Books Ltd/ })).toBeTruthy();
      const wholeYear = within(screen.getByRole("region", { name: "Whole year" }));
      expect(wholeYear.getAllByText("€100.00")).toHaveLength(2);
      expect(wholeYear.getByRole("button", { name: /Sample Laptop/ })).toBeTruthy();
    });

    it("says nothing counts yet when the year has no months", async () => {
      await show({ months: [] });

      await byMonth();

      expect(screen.getByText("Nothing counts in 2026 yet.")).toBeTruthy();
    });

    it("goes back to the Kennzahlen", async () => {
      await show();
      await byMonth();

      await userEvent.click(screen.getByRole("button", { name: "By Kennzahl" }));

      expect(screen.getByRole("region", { name: "KZ 720 Fachliteratur" })).toBeTruthy();
    });
  });

  describe("More menu", () => {
    const openMenu = () => userEvent.click(screen.getByRole("button", { name: "More" }));

    it("opens the Log of the year", async () => {
      await show();
      await openMenu();

      await userEvent.click(screen.getByRole("menuitem", { name: "Log" }));

      expect(useStore.getState().route).toEqual({ page: "log", year: 2026 });
    });

    it("opens the Archive of the year and shows how many documents it holds", async () => {
      await show({ archived: 3 });
      await openMenu();
      const archive = screen.getByRole("menuitem", { name: /^Archive/ });
      expect(within(archive).getByText("3")).toBeTruthy();

      await userEvent.click(archive);

      expect(useStore.getState().route).toEqual({ page: "archive", year: 2026 });
    });

    it("keeps Export Summary PDF disabled", async () => {
      await show();
      await openMenu();

      expect(screen.getByRole<HTMLButtonElement>("menuitem", { name: "Export Summary PDF…" }).disabled).toBe(true);
    });

    it("opens the ruleset JSON of the year and closes the menu", async () => {
      await show();
      await openMenu();

      await userEvent.click(screen.getByRole("menuitem", { name: "Open Ruleset JSON" }));

      expect(mocked.rulesOpen).toHaveBeenCalledWith(2026);
      expect(screen.queryByRole("menu")).toBeNull();
    });
  });
});
