import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { useStore } from "../state/store";
import { attachEvidence, deleteEntry, openEvidence } from "./entryActions";

const mocked = vi.mocked(api, true);

const toasts = () => useStore.getState().toasts.map((t) => [t.kind, t.message]);
const failure = { kind: "invalid_input", message: "Tax year 2026 is closed", code: null };

beforeEach(() => {
  vi.clearAllMocks();
  useStore.setState({ toasts: [] });
});

describe("deleteEntry", () => {
  it("deletes the entry of the year and offers Undo", async () => {
    mocked.yearEntryDelete.mockResolvedValue("Delete entry");

    deleteEntry(2026, "entry-1");

    await vi.waitFor(() => expect(toasts()).toEqual([["success", "Delete entry"]]));
    expect(mocked.yearEntryDelete).toHaveBeenCalledWith(2026, "entry-1");
    expect(useStore.getState().toasts[0].action?.label).toBe("Undo");
  });

  it("shows an error toast when it fails", async () => {
    mocked.yearEntryDelete.mockRejectedValue(failure);

    deleteEntry(2026, "entry-1");

    await vi.waitFor(() => expect(toasts()).toEqual([["error", "Tax year 2026 is closed"]]));
  });
});

describe("attachEvidence", () => {
  it("attaches the chosen PDF to the entry and offers Undo", async () => {
    mocked.yearEvidenceChoose.mockResolvedValue("/Users/demo/Documents/receipt.pdf");
    mocked.yearEntryAttach.mockResolvedValue("Attach PDF");

    attachEvidence(2026, "entry-1");

    await vi.waitFor(() => expect(toasts()).toEqual([["success", "Attach PDF"]]));
    expect(mocked.yearEntryAttach).toHaveBeenCalledWith(2026, "entry-1", "/Users/demo/Documents/receipt.pdf");
    expect(useStore.getState().toasts[0].action?.label).toBe("Undo");
  });

  it("does nothing when the file dialog is cancelled", async () => {
    mocked.yearEvidenceChoose.mockResolvedValue(null);

    attachEvidence(2026, "entry-1");

    await vi.waitFor(() => expect(mocked.yearEvidenceChoose).toHaveBeenCalled());
    await Promise.resolve();
    expect(mocked.yearEntryAttach).not.toHaveBeenCalled();
    expect(toasts()).toEqual([]);
  });

  it("shows an error toast when the file dialog fails", async () => {
    mocked.yearEvidenceChoose.mockRejectedValue({ kind: "io", message: "Dialog failed", code: null });

    attachEvidence(2026, "entry-1");

    await vi.waitFor(() => expect(toasts()).toEqual([["error", "Dialog failed"]]));
  });

  it("shows an error toast when attaching fails", async () => {
    mocked.yearEvidenceChoose.mockResolvedValue("/Users/demo/Documents/receipt.pdf");
    mocked.yearEntryAttach.mockRejectedValue(failure);

    attachEvidence(2026, "entry-1");

    await vi.waitFor(() => expect(toasts()).toEqual([["error", "Tax year 2026 is closed"]]));
  });
});

describe("openEvidence", () => {
  it("asks Rust to open the entry's PDF and shows nothing on success", async () => {
    openEvidence(2026, "entry-1");

    await vi.waitFor(() => expect(mocked.yearEvidenceOpen).toHaveBeenCalledWith(2026, "entry-1"));
    expect(toasts()).toEqual([]);
  });

  it("shows an error toast when the PDF cannot be opened", async () => {
    mocked.yearEvidenceOpen.mockRejectedValue({ kind: "not_found", message: "PDF is missing", code: null });

    openEvidence(2026, "entry-1");

    await vi.waitFor(() => expect(toasts()).toEqual([["error", "PDF is missing"]]));
  });
});
