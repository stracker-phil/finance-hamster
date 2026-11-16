import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { queryClient } from "../queries/queryClient";
import { HOME } from "../state/route";
import { useStore } from "../state/store";
import { addPreview } from "../test/documents";
import { settings } from "../test/settings";
import { runPageCommand } from "./pageCommands";

const mocked = vi.mocked(api, true);

beforeEach(() => {
  vi.clearAllMocks();
  useStore.setState({ route: HOME, back: [], forward: [], toasts: [], addPreview: null, addKind: "invoice" });
  mocked.updateSettings.mockResolvedValue(settings());
});

describe("runPageCommand", () => {
  it("opens the Review page for go:review", () => {
    runPageCommand("go:review");

    expect(useStore.getState().route).toEqual({ page: "review" });
  });

  describe("go:year:*", () => {
    const taxYears = [2027, 2026, 2025].map((year) => ({ year, state: "open" as const, ruleset: `${year} v1` }));

    beforeEach(() => {
      queryClient.clear();
      mocked.listTaxYears.mockResolvedValue(taxYears);
    });

    it.each([
      ["go:year:newest", 2027],
      ["go:year:second", 2026],
      ["go:year:third", 2025],
    ])("opens the tax year at that place of the newest-first list for %s", async (id, year) => {
      runPageCommand(id);

      await vi.waitFor(() => expect(useStore.getState().route).toEqual({ page: "year", year }));
    });

    it("changes nothing when there are fewer tax years", async () => {
      mocked.listTaxYears.mockResolvedValue(taxYears.slice(0, 1));

      runPageCommand("go:year:second");

      await vi.waitFor(() => expect(mocked.listTaxYears).toHaveBeenCalled());
      await new Promise((resolve) => setTimeout(resolve, 0));
      expect(useStore.getState().route).toEqual(HOME);
    });
  });

  it("opens the Add document sheet for the invoice the owner chose for document:add", async () => {
    const preview = addPreview();
    mocked.documentAddChoose.mockResolvedValue(preview);

    runPageCommand("document:add");

    await vi.waitFor(() => expect(useStore.getState().addPreview).toEqual(preview));
  });

  it("opens the Add document sheet on the invoice kind for document:add", async () => {
    const preview = addPreview();
    mocked.documentAddChoose.mockResolvedValue(preview);
    useStore.setState({ addKind: "other" });

    runPageCommand("document:add");

    await vi.waitFor(() => expect(useStore.getState().addPreview).toEqual(preview));
    expect(useStore.getState().addKind).toBe("invoice");
  });

  it.each([
    ["payslip:add", "payslip"],
    ["other:add", "other"],
  ] as const)("opens the Add document sheet on the %s kind for the PDF the owner chose", async (id, kind) => {
    const preview = addPreview();
    mocked.documentAddChoose.mockResolvedValue(preview);

    runPageCommand(id);

    await vi.waitFor(() => expect(useStore.getState().addPreview).toEqual(preview));
    expect(useStore.getState().addKind).toBe(kind);
  });

  it.each(["payslip:add", "other:add"])("opens no sheet for %s when the file dialog is cancelled", async (id) => {
    mocked.documentAddChoose.mockResolvedValue(null);

    runPageCommand(id);

    await vi.waitFor(() => expect(mocked.documentAddChoose).toHaveBeenCalled());
    await Promise.resolve();
    expect(useStore.getState().addPreview).toBeNull();
  });

  describe("view:payslips", () => {
    it.each([
      [false, true],
      [true, false],
    ])("saves the payslip panel as %s to %s", async (current, next) => {
      mocked.getSettings.mockResolvedValue(settings({ payslipPanel: current }));

      runPageCommand("view:payslips");

      await vi.waitFor(() => expect(mocked.updateSettings).toHaveBeenCalledWith({ payslipPanel: next }));
    });

    it("reports a setting that cannot be saved as an error toast", async () => {
      mocked.getSettings.mockResolvedValue(settings());
      mocked.updateSettings.mockRejectedValue({ kind: "io", message: "Disk full", code: null });

      runPageCommand("view:payslips");

      await vi.waitFor(() => expect(useStore.getState().toasts.map((t) => t.message)).toEqual(["Disk full"]));
    });
  });
});
