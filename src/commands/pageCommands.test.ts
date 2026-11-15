import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { HOME } from "../state/route";
import { useStore } from "../state/store";
import { addPreview } from "../test/documents";
import { settings } from "../test/settings";
import { runPageCommand } from "./pageCommands";

const mocked = vi.mocked(api, true);

beforeEach(() => {
  vi.clearAllMocks();
  useStore.setState({ route: HOME, back: [], forward: [], toasts: [], addPreview: null });
  mocked.updateSettings.mockResolvedValue(settings());
});

describe("runPageCommand", () => {
  it("opens the Review page for go:review", () => {
    runPageCommand("go:review");

    expect(useStore.getState().route).toEqual({ page: "review" });
  });

  it("opens the Add document sheet for the invoice the owner chose for document:add", async () => {
    const preview = addPreview();
    mocked.documentAddChoose.mockResolvedValue(preview);

    runPageCommand("document:add");

    await vi.waitFor(() => expect(useStore.getState().addPreview).toEqual(preview));
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
