import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../../test/apiMock")).createApiMock() }));

import { api, type RulesFile } from "../../api";
import { useStore } from "../../state/store";
import { renderWithQuery } from "../../test/renderWithQuery";
import { reloadPreview, rulesFile } from "../../test/rules";
import { TaxRulesPage, describeFile } from "./TaxRulesPage";

const mocked = vi.mocked(api, true);

const toasts = () => useStore.getState().toasts.map((t) => ({ kind: t.kind, message: t.message }));
const button = (name: string) => screen.getByRole("button", { name });

const OPEN_2026 = rulesFile();
const CLOSED_2025 = rulesFile({ fileName: "rules-2025.json", year: 2025, state: "closed", closedOn: "14 Mar 2026" });

beforeEach(() => {
  vi.clearAllMocks();
  useStore.setState({ toasts: [] });
  mocked.rulesOverview.mockResolvedValue([OPEN_2026, CLOSED_2025]);
  mocked.rulesReloadPreview.mockResolvedValue([]);
  mocked.rulesReloadApply.mockResolvedValue(null);
  mocked.rulesImport.mockResolvedValue(null);
  mocked.rulesDuplicate.mockResolvedValue("rules-2027.json");
  mocked.rulesOpen.mockResolvedValue(undefined);
  mocked.rulesShowFolder.mockResolvedValue(undefined);
});

describe("describeFile", () => {
  it.each<[string, Partial<RulesFile>, string]>([
    ["a draft duplicated from another file", { state: "draft", basedOn: "rules-2026.json" }, "Duplicated from rules-2026.json · not used yet"],
    ["a draft that is the next year", { state: "draft", basedOn: null }, "Next year · not used yet"],
    ["a draft with a problem", { state: "draft", problems: ["bad json"] }, "Next year · not used yet · can't be used: bad json"],
    ["a valid open year", {}, "Tax Year 2026 · valid · 12 items"],
    ["an open year with unverified items", { unverified: 3 }, "Tax Year 2026 · valid · 12 items · 3 unverified"],
    ["an edited open year", { edited: true }, "Tax Year 2026 · valid · 12 items · edited, reload to apply"],
    ["an open year whose file is missing", { missing: true }, "The file is missing; the year keeps the rules it loaded"],
    ["an open year with one problem", { problems: ["items[0].kz is missing"], edited: true }, "Tax Year 2026 · can't be used: items[0].kz is missing"],
    ["an open year with several problems", { problems: ["first", "second", "third"] }, "Tax Year 2026 · can't be used: first (and 2 more)"],
    ["a closed year", { state: "closed", year: 2025, closedOn: "14 Mar 2026" }, "Frozen with Tax Year 2025 on 14 Mar 2026 · read only"],
    ["a closed year without a closing date", { state: "closed", year: 2025, closedOn: null }, "Frozen with Tax Year 2025 · read only"],
    ["a closed year with problems in its file", { state: "closed", year: 2025, closedOn: "14 Mar 2026", problems: ["bad"] }, "Frozen with Tax Year 2025 on 14 Mar 2026 · read only"],
  ])("describes %s", (_name, patch, expected) => {
    expect(describeFile(rulesFile(patch))).toBe(expected);
  });
});

describe("TaxRulesPage", () => {
  describe("rulesets", () => {
    it("lists each file with its description and state pill", async () => {
      mocked.rulesOverview.mockResolvedValue([OPEN_2026, CLOSED_2025, rulesFile({ fileName: "rules-2027.json", year: 2027, state: "draft" })]);
      renderWithQuery(<TaxRulesPage />);

      expect(await screen.findByText("Tax Year 2026 · valid · 12 items")).toBeTruthy();
      expect(screen.getByText("Frozen with Tax Year 2025 on 14 Mar 2026 · read only")).toBeTruthy();
      expect(screen.getByText("Next year · not used yet")).toBeTruthy();
      for (const pill of ["Open", "Closed", "Draft"]) expect(screen.getByText(pill)).toBeTruthy();
    });

    it("offers View for a closed year and Open in Editor otherwise", async () => {
      renderWithQuery(<TaxRulesPage />);
      await screen.findByText("rules-2026.json");

      expect(screen.getAllByRole("button", { name: "Open in Editor" })).toHaveLength(1);
      expect(screen.getAllByRole("button", { name: "View" })).toHaveLength(1);
    });

    it.each([
      ["Open in Editor", 2026],
      ["View", 2025],
    ])("opens the file of the year when %s is clicked", async (name, year) => {
      renderWithQuery(<TaxRulesPage />);
      await screen.findByText("rules-2026.json");

      await userEvent.click(button(name));

      expect(mocked.rulesOpen).toHaveBeenCalledWith(year);
    });

    it("disables Open in Editor for an open year whose file is missing", async () => {
      mocked.rulesOverview.mockResolvedValue([rulesFile({ missing: true })]);
      renderWithQuery(<TaxRulesPage />);

      await screen.findByText("The file is missing; the year keeps the rules it loaded");

      expect(button("Open in Editor")).toBeDisabled();
    });

    it("keeps View enabled for a closed year whose file is missing", async () => {
      mocked.rulesOverview.mockResolvedValue([{ ...CLOSED_2025, missing: true }]);
      renderWithQuery(<TaxRulesPage />);
      await screen.findByText("rules-2025.json");

      expect(button("View")).toBeEnabled();
    });

    it("shows the error as a toast when the file cannot be opened", async () => {
      mocked.rulesOpen.mockRejectedValue({ kind: "io", message: "No editor available", code: null });
      renderWithQuery(<TaxRulesPage />);
      await screen.findByText("rules-2026.json");

      await userEvent.click(button("Open in Editor"));

      await waitFor(() => expect(toasts()).toEqual([{ kind: "error", message: "No editor available" }]));
    });
  });

  describe("reload", () => {
    it("says what was edited in the row's description", async () => {
      mocked.rulesOverview.mockResolvedValue([rulesFile({ edited: true })]);
      renderWithQuery(<TaxRulesPage />);

      expect(await screen.findByText("rules-2026.json was edited. Shows what changes before applying.")).toBeTruthy();
    });

    it("names the broken file in the row's description", async () => {
      mocked.rulesOverview.mockResolvedValue([rulesFile({ problems: ["bad json"] })]);
      renderWithQuery(<TaxRulesPage />);

      expect(await screen.findByText("rules-2026.json can't be used: bad json")).toBeTruthy();
    });

    it("shows the last load date when nothing was edited", async () => {
      mocked.rulesOverview.mockResolvedValue([rulesFile({ loadedOn: "2 Jan 2026" })]);
      renderWithQuery(<TaxRulesPage />);

      expect(await screen.findByText("Last loaded 2 Jan 2026 · no errors. Shows what changes in the open year before applying.")).toBeTruthy();
    });

    it("toasts that nothing changed when no file has", async () => {
      renderWithQuery(<TaxRulesPage />);

      await userEvent.click(button("Reload…"));

      await waitFor(() => expect(toasts()).toEqual([{ kind: "info", message: "No rules file changed since it was loaded" }]));
      expect(screen.queryByRole("dialog")).toBeNull();
    });

    it("opens the Reload sheet with the changes when a file has changed", async () => {
      mocked.rulesReloadPreview.mockResolvedValue([reloadPreview()]);
      renderWithQuery(<TaxRulesPage />);

      await userEvent.click(button("Reload…"));

      expect(await screen.findByRole("dialog", { name: "Reload Tax Rules" })).toBeTruthy();
      expect(screen.getByText("Home office limit changed")).toBeTruthy();
    });

    it("shows the error as a toast when the preview fails", async () => {
      mocked.rulesReloadPreview.mockRejectedValue({ kind: "io", message: "Cannot read rules", code: null });
      renderWithQuery(<TaxRulesPage />);

      await userEvent.click(button("Reload…"));

      await waitFor(() => expect(toasts()).toEqual([{ kind: "error", message: "Cannot read rules" }]));
    });
  });

  describe("import", () => {
    it("does nothing when the file panel is cancelled", async () => {
      renderWithQuery(<TaxRulesPage />);

      await userEvent.click(button("Import…"));

      await waitFor(() => expect(mocked.rulesImport).toHaveBeenCalled());
      expect(toasts()).toEqual([]);
      expect(mocked.rulesReloadPreview).not.toHaveBeenCalled();
    });

    it("toasts the imported file and the backup of the replaced one", async () => {
      mocked.rulesImport.mockResolvedValue({ fileName: "rules-2027.json", year: 2027, backup: "rules-2027.1.json", needsReload: false });
      renderWithQuery(<TaxRulesPage />);

      await userEvent.click(button("Import…"));

      await waitFor(() => expect(toasts()).toEqual([{ kind: "success", message: "Imported rules-2027.json; the old file is rules-2027.1.json" }]));
      expect(mocked.rulesReloadPreview).not.toHaveBeenCalled();
    });

    it("toasts the imported file alone when nothing was replaced", async () => {
      mocked.rulesImport.mockResolvedValue({ fileName: "rules-2027.json", year: 2027, backup: null, needsReload: false });
      renderWithQuery(<TaxRulesPage />);

      await userEvent.click(button("Import…"));

      await waitFor(() => expect(toasts()).toEqual([{ kind: "success", message: "Imported rules-2027.json" }]));
    });

    it("shows what the reload changes when the imported year is open", async () => {
      mocked.rulesImport.mockResolvedValue({ fileName: "rules-2026.json", year: 2026, backup: null, needsReload: true });
      mocked.rulesReloadPreview.mockResolvedValue([reloadPreview()]);
      renderWithQuery(<TaxRulesPage />);

      await userEvent.click(button("Import…"));

      expect(await screen.findByRole("dialog", { name: "Reload Tax Rules" })).toBeTruthy();
    });
  });

  describe("duplicate", () => {
    it("is disabled when the next year's file exists", async () => {
      mocked.rulesOverview.mockResolvedValue([OPEN_2026, rulesFile({ fileName: "rules-2027.json", year: 2027, state: "draft" })]);
      renderWithQuery(<TaxRulesPage />);

      expect(await screen.findByText("rules-2027.json exists.")).toBeTruthy();
      expect(button("Duplicate…")).toBeDisabled();
    });

    it("is disabled when no year is open", async () => {
      mocked.rulesOverview.mockResolvedValue([CLOSED_2025]);
      renderWithQuery(<TaxRulesPage />);

      expect(await screen.findByText("Needs an open tax year to copy from.")).toBeTruthy();
      expect(button("Duplicate…")).toBeDisabled();
    });

    it("opens a sheet for the year after the open one and cancels without copying", async () => {
      renderWithQuery(<TaxRulesPage />);
      await waitFor(() => expect(button("Duplicate…")).toBeEnabled());

      await userEvent.click(button("Duplicate…"));
      expect(screen.getByRole("dialog", { name: "Start 2027" })).toBeTruthy();
      await userEvent.click(button("Cancel"));

      expect(screen.queryByRole("dialog")).toBeNull();
      expect(mocked.rulesDuplicate).not.toHaveBeenCalled();
    });

    it("creates the draft, toasts its file name and closes the sheet", async () => {
      renderWithQuery(<TaxRulesPage />);
      await waitFor(() => expect(button("Duplicate…")).toBeEnabled());
      await userEvent.click(button("Duplicate…"));

      await userEvent.click(button("Duplicate"));

      await waitFor(() => expect(toasts()).toEqual([{ kind: "success", message: "Created rules-2027.json as a draft" }]));
      expect(screen.queryByRole("dialog")).toBeNull();
    });

    it("shows the error as a toast and closes the sheet when copying fails", async () => {
      mocked.rulesDuplicate.mockRejectedValue({ kind: "io", message: "File exists", code: null });
      renderWithQuery(<TaxRulesPage />);
      await waitFor(() => expect(button("Duplicate…")).toBeEnabled());
      await userEvent.click(button("Duplicate…"));

      await userEvent.click(button("Duplicate"));

      await waitFor(() => expect(toasts()).toEqual([{ kind: "error", message: "File exists" }]));
      expect(screen.queryByRole("dialog")).toBeNull();
    });
  });

  describe("rules folder", () => {
    it("shows the folder under the data folder", async () => {
      renderWithQuery(<TaxRulesPage />);

      expect(await screen.findByText("~/Library/Application Support/com.example.hamster/rules/")).toBeTruthy();
    });

    it("reveals it in Finder", async () => {
      renderWithQuery(<TaxRulesPage />);

      await userEvent.click(button("Show in Finder"));

      expect(mocked.rulesShowFolder).toHaveBeenCalled();
    });
  });
});
