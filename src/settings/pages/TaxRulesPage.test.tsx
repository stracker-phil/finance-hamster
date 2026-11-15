import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../../test/apiMock")).createApiMock() }));

import { api, type RulesImport, type RulesYear } from "../../api";
import { ConfirmHost } from "../../components/ConfirmSheet";
import { setLanguage } from "../../i18n";
import { useStore } from "../../state/store";
import { renderWithQuery } from "../../test/renderWithQuery";
import { rulesOverview, rulesYear, rulesYearView } from "../../test/rules";
import { SettingsOpen } from "../controls";
import { TaxRulesPage, describeYear } from "./TaxRulesPage";

const mocked = vi.mocked(api, true);

const toasts = () => useStore.getState().toasts.map((t) => ({ kind: t.kind, message: t.message }));

const CLOSED_2025 = rulesYear({ year: 2025, state: "closed", ruleset: "2025 v2" });

const found = (patch: Partial<RulesImport> = {}): RulesImport => ({ year: 2029, text: '{"year":2029}', replaces: false, problems: [], changes: [], ...patch });

function renderPage() {
  return renderWithQuery(
    <>
      <TaxRulesPage />
      <ConfirmHost />
    </>,
  );
}

beforeEach(() => {
  vi.clearAllMocks();
  useStore.setState({ toasts: [], confirmation: null });
  mocked.rulesOverview.mockResolvedValue(rulesOverview([rulesYear(), CLOSED_2025]));
  mocked.rulesYear.mockResolvedValue(rulesYearView());
  mocked.rulesStartNext.mockResolvedValue("Start 2027");
});

describe("describeYear", () => {
  it.each<[string, Partial<RulesYear>, string]>([
    ["an open year on the built-in rules", {}, "Built-in rules 2026 v1 · open"],
    ["an open year on the owner's version", { source: "own", basedOn: "2026 v1" }, "Your version, based on 2026 v1 · open"],
    ["an owner's version without a base", { source: "own", basedOn: null }, "Your version · open"],
    ["a closed year", { state: "closed", ruleset: "2025 v2" }, "Built-in rules 2025 v2 · closed, frozen"],
    ["a year with a newer built-in version", { source: "own", basedOn: "2026 v1", builtinNewer: "2026 v2" }, "Your version, based on 2026 v1 · open · built-in 2026 v2 is newer"],
    [
      "a year whose newer built-in version can't be taken on",
      { source: "own", basedOn: "2026 v1", builtinNewer: "2026 v2", problems: ["items[0].kz is missing", "other"] },
      "Your version, based on 2026 v1 · open · built-in 2026 v2 can't be taken on: items[0].kz is missing",
    ],
    ["a year with unverified items", { unverified: 3 }, "Built-in rules 2026 v1 · open · 3 unverified"],
  ])("describes %s", (_name, patch, expected) => {
    expect(describeYear(rulesYear(patch))).toBe(expected);
  });
});

describe("TaxRulesPage", () => {
  describe("years", () => {
    it("lists each year with where its rules come from and its state", async () => {
      renderPage();

      expect(await screen.findByText("Built-in rules 2026 v1 · open")).toBeTruthy();
      expect(screen.getByText("Built-in rules 2025 v2 · closed, frozen")).toBeTruthy();
    });

    it.each([
      ["an open year", /^2026.*Built-in rules 2026 v1 · open/, "taxrules/2026"],
      ["a closed year", /^2025.*Built-in rules 2025 v2 · closed, frozen/, "taxrules/2025"],
    ])("opens %s one level down when its row is clicked", async (_name, name, target) => {
      const onOpen = vi.fn();
      renderWithQuery(
        <SettingsOpen.Provider value={onOpen}>
          <TaxRulesPage />
        </SettingsOpen.Provider>,
      );

      await userEvent.click(await screen.findByRole("button", { name }));

      expect(onOpen).toHaveBeenCalledWith(target);
    });

    it("lists the years newest first as given", async () => {
      renderPage();
      await screen.findByText("Built-in rules 2026 v1 · open");

      const years = screen.getAllByRole("button", { name: /^20\d\d/ }).map((b) => b.textContent?.slice(0, 4));
      expect(years).toEqual(["2026", "2025"]);
    });
  });

  describe("start", () => {
    it("offers no Start row when there is no next year to begin", async () => {
      renderPage();
      await screen.findByText("Built-in rules 2026 v1 · open");

      expect(screen.queryByRole("button", { name: "Start…" })).toBeNull();
    });

    it("says it copies the newest year's rules when the next year has no built-in rules", async () => {
      mocked.rulesOverview.mockResolvedValue(rulesOverview([rulesYear()], { nextYear: 2027, nextIsBuiltin: false }));
      renderPage();

      expect(await screen.findByText("Start 2027")).toBeTruthy();
      expect(screen.getByText("Copies the rules of 2026. Check every value against the new year's forms.")).toBeTruthy();
    });

    it("says it uses the built-in rules when the next year has some", async () => {
      mocked.rulesOverview.mockResolvedValue(rulesOverview([rulesYear()], { nextYear: 2027, nextIsBuiltin: true }));
      renderPage();

      expect(await screen.findByText("Uses the rules built into Hamster.")).toBeTruthy();
    });

    it("puts the Start row above the years", async () => {
      mocked.rulesOverview.mockResolvedValue(rulesOverview([rulesYear(), CLOSED_2025], { nextYear: 2027, nextIsBuiltin: true }));
      renderPage();

      const start = await screen.findByText("Start 2027");
      const firstYear = screen.getByRole("button", { name: /^2026/ });
      expect(start.compareDocumentPosition(firstYear) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    });

    describe("with a next year", () => {
      beforeEach(() => {
        mocked.rulesOverview.mockResolvedValue(rulesOverview([rulesYear()], { nextYear: 2027, nextIsBuiltin: true }));
      });

      it("asks before starting and starts the next year once confirmed", async () => {
        renderPage();
        await userEvent.click(await screen.findByRole("button", { name: "Start…" }));

        const confirm = await screen.findByRole("alertdialog", { name: "Start 2027" });
        expect(mocked.rulesStartNext).not.toHaveBeenCalled();
        await userEvent.click(within(confirm).getByRole("button", { name: "Start" }));

        await waitFor(() => expect(toasts()).toEqual([{ kind: "success", message: "Start 2027" }]));
        expect(mocked.rulesStartNext).toHaveBeenCalled();
      });

      it("starts nothing when the question is cancelled", async () => {
        renderPage();
        await userEvent.click(await screen.findByRole("button", { name: "Start…" }));

        const confirm = await screen.findByRole("alertdialog");
        await userEvent.click(within(confirm).getByRole("button", { name: "Cancel" }));

        expect(screen.queryByRole("alertdialog")).toBeNull();
        expect(mocked.rulesStartNext).not.toHaveBeenCalled();
      });

      it("shows the error as a toast when starting fails", async () => {
        mocked.rulesStartNext.mockRejectedValue({ kind: "invalid_input", message: "No rules for 2027", code: null });
        renderPage();
        await userEvent.click(await screen.findByRole("button", { name: "Start…" }));

        await userEvent.click(within(await screen.findByRole("alertdialog")).getByRole("button", { name: "Start" }));

        await waitFor(() => expect(toasts()).toEqual([{ kind: "error", message: "No rules for 2027" }]));
      });
    });
  });

  describe("import", () => {
    afterEach(() => setLanguage("en"));

    const chooseFile = async () => {
      renderPage();
      await userEvent.click(await screen.findByRole("button", { name: "Import…" }));
    };

    it("shows the Import JSON row below the years", async () => {
      renderPage();

      const lastYear = await screen.findByRole("button", { name: /^2025/ });
      const row = screen.getByText("Import JSON");
      expect(lastYear.compareDocumentPosition(row) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
      expect(screen.getByRole("button", { name: "Import…" })).toBeTruthy();
    });

    it("imports nothing and shows no toast when the file panel is cancelled", async () => {
      mocked.rulesImportChoose.mockResolvedValue(null);
      await chooseFile();

      await waitFor(() => expect(mocked.rulesImportChoose).toHaveBeenCalled());
      expect(screen.queryByRole("alertdialog")).toBeNull();
      expect(mocked.rulesImport).not.toHaveBeenCalled();
      expect(toasts()).toEqual([]);
    });

    it.each([
      ["lists the problems of a file that has some", { problems: ["a", "b"] }, "This file can't be imported: a; b"],
      ["reports a file without a year", { year: null, problems: ["year is missing"] }, "This file can't be imported: year is missing"],
    ])("%s as an error toast without asking or importing", async (_name, patch, message) => {
      mocked.rulesImportChoose.mockResolvedValue(found(patch));
      await chooseFile();

      await waitFor(() => expect(toasts()).toEqual([{ kind: "error", message }]));
      expect(screen.queryByRole("alertdialog")).toBeNull();
      expect(mocked.rulesImport).not.toHaveBeenCalled();
    });

    it("says there is nothing to import when the file matches the rules of the year it replaces", async () => {
      mocked.rulesImportChoose.mockResolvedValue(found({ year: 2026, replaces: true, changes: [] }));
      await chooseFile();

      await waitFor(() => expect(toasts()).toEqual([{ kind: "info", message: "The file matches the rules of 2026; nothing to import." }]));
      expect(screen.queryByRole("alertdialog")).toBeNull();
      expect(mocked.rulesImport).not.toHaveBeenCalled();
    });

    it("asks before starting a new year from the file and imports its text once confirmed, with an undo toast", async () => {
      mocked.rulesImportChoose.mockResolvedValue(found({ text: '{"year":2029,"items":[]}' }));
      mocked.rulesImport.mockResolvedValue("Imported the Tax Rules for 2029");
      await chooseFile();

      const confirm = await screen.findByRole("alertdialog", { name: "Import the Tax Rules for 2029?" });
      expect(within(confirm).getByText("Starts tax year 2029 with the rules from the file. You can undo it.")).toBeTruthy();
      expect(mocked.rulesImport).not.toHaveBeenCalled();
      await userEvent.click(within(confirm).getByRole("button", { name: "Import" }));

      await waitFor(() => expect(toasts()).toEqual([{ kind: "success", message: "Imported the Tax Rules for 2029" }]));
      expect(mocked.rulesImport).toHaveBeenCalledWith('{"year":2029,"items":[]}');
      expect(useStore.getState().toasts[0].action?.label).toBe("Undo");
    });

    it("lists what changes when the file replaces the rules of a year", async () => {
      mocked.rulesImportChoose.mockResolvedValue(found({ year: 2026, replaces: true, changes: ["X", "Y"] }));
      await chooseFile();

      const confirm = await screen.findByRole("alertdialog", { name: "Import the Tax Rules for 2026?" });
      expect(within(confirm).getByText("Replaces the rules of 2026: X; Y. You can undo it.")).toBeTruthy();
    });

    it("imports nothing when the question is declined", async () => {
      mocked.rulesImportChoose.mockResolvedValue(found());
      await chooseFile();

      await userEvent.click(within(await screen.findByRole("alertdialog")).getByRole("button", { name: "Cancel" }));

      expect(screen.queryByRole("alertdialog")).toBeNull();
      expect(mocked.rulesImport).not.toHaveBeenCalled();
      expect(toasts()).toEqual([]);
    });

    it("shows the error as a toast when importing fails", async () => {
      mocked.rulesImportChoose.mockResolvedValue(found());
      mocked.rulesImport.mockRejectedValue({ kind: "invalid_input", message: "Year 2029 is closed", code: null });
      await chooseFile();

      await userEvent.click(within(await screen.findByRole("alertdialog")).getByRole("button", { name: "Import" }));

      await waitFor(() => expect(toasts()).toEqual([{ kind: "error", message: "Year 2029 is closed" }]));
    });

    it("shows an error toast when the file panel fails", async () => {
      mocked.rulesImportChoose.mockRejectedValue({ kind: "io", message: "Can't read the file", code: null });
      await chooseFile();

      await waitFor(() => expect(toasts()).toEqual([{ kind: "error", message: "Can't read the file" }]));
    });

    it("words the row and the confirmation in German", async () => {
      setLanguage("de");
      mocked.rulesImportChoose.mockResolvedValue(found());
      renderPage();

      expect(await screen.findByText("JSON importieren")).toBeTruthy();
      await userEvent.click(screen.getByRole("button", { name: "Importieren …" }));

      const confirm = await screen.findByRole("alertdialog", { name: "Steuerregeln für 2029 importieren?" });
      expect(within(confirm).getByRole("button", { name: "Importieren" })).toBeTruthy();
    });
  });
});
