import { screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { HOME } from "../state/route";
import { useStore } from "../state/store";
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
