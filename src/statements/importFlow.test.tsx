import { act, render, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api, type ImportReady } from "../api";
import { useStore } from "../state/store";
import { importPreview } from "../test/statements";
import { startImport, useDroppedStatements } from "./importFlow";

const mocked = vi.mocked(api, true);

beforeEach(() => {
  vi.clearAllMocks();
  useStore.setState({ importPreview: null, toasts: [] });
  mocked.listen.mockResolvedValue(() => {});
});

describe("startImport", () => {
  it("puts the statement the owner chose into the store for the Import sheet", async () => {
    const preview = importPreview();
    mocked.statementImportChoose.mockResolvedValue(preview);

    startImport();

    await waitFor(() => expect(useStore.getState().importPreview).toEqual(preview));
  });

  it("does nothing when the file dialog is cancelled", async () => {
    mocked.statementImportChoose.mockResolvedValue(null);

    startImport();

    await waitFor(() => expect(mocked.statementImportChoose).toHaveBeenCalled());
    await Promise.resolve();
    expect(useStore.getState().importPreview).toBeNull();
    expect(useStore.getState().toasts).toEqual([]);
  });

  it("shows an error toast when the statement cannot be read", async () => {
    mocked.statementImportChoose.mockRejectedValue({ kind: "invalid_input", message: "Not a text PDF", code: null });

    startImport();

    await waitFor(() => expect(useStore.getState().toasts.map((t) => [t.kind, t.message])).toEqual([["error", "Not a text PDF"]]));
    expect(useStore.getState().importPreview).toBeNull();
  });
});

describe("useDroppedStatements", () => {
  function Probe() {
    useDroppedStatements();
    return null;
  }

  async function drop(payload: ImportReady) {
    await waitFor(() => expect(mocked.listen.mock.calls.some(([name]) => name === "import_ready")).toBe(true));
    const handler = mocked.listen.mock.calls.find(([name]) => name === "import_ready")![1] as (payload: ImportReady) => void;
    act(() => handler(payload));
  }

  it("puts a dropped statement that was read into the store", async () => {
    const preview = importPreview();
    render(<Probe />);

    await drop({ preview, error: null });

    expect(useStore.getState().importPreview).toEqual(preview);
  });

  it("shows why a dropped file cannot be imported as an error toast", async () => {
    render(<Probe />);

    await drop({ preview: null, error: "No account matches this IBAN" });

    expect(useStore.getState().toasts.map((t) => [t.kind, t.message])).toEqual([["error", "No account matches this IBAN"]]);
    expect(useStore.getState().importPreview).toBeNull();
  });

  it("stops listening on unmount", async () => {
    const unlisten = vi.fn();
    mocked.listen.mockResolvedValue(unlisten);
    const { unmount } = render(<Probe />);
    await waitFor(() => expect(mocked.listen).toHaveBeenCalled());

    unmount();

    await waitFor(() => expect(unlisten).toHaveBeenCalled());
  });
});
