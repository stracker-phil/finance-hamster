import { act, render, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api, type AddReady } from "../api";
import { useStore } from "../state/store";
import { addPreview } from "../test/documents";
import { startAddDocument, useDroppedDocuments } from "./addFlow";

const mocked = vi.mocked(api, true);

beforeEach(() => {
  vi.clearAllMocks();
  useStore.setState({ addPreview: null, toasts: [] });
  mocked.listen.mockResolvedValue(() => {});
});

describe("startAddDocument", () => {
  it("puts the invoice the owner chose into the store for the Add document sheet", async () => {
    const preview = addPreview();
    mocked.documentAddChoose.mockResolvedValue(preview);

    startAddDocument();

    await waitFor(() => expect(useStore.getState().addPreview).toEqual(preview));
  });

  it("does nothing when the file dialog is cancelled", async () => {
    mocked.documentAddChoose.mockResolvedValue(null);

    startAddDocument();

    await waitFor(() => expect(mocked.documentAddChoose).toHaveBeenCalled());
    await Promise.resolve();
    expect(useStore.getState().addPreview).toBeNull();
    expect(useStore.getState().toasts).toEqual([]);
  });

  it("shows an error toast when the invoice cannot be read", async () => {
    mocked.documentAddChoose.mockRejectedValue({ kind: "invalid_input", message: "Not a text PDF", code: null });

    startAddDocument();

    await waitFor(() => expect(useStore.getState().toasts.map((t) => [t.kind, t.message])).toEqual([["error", "Not a text PDF"]]));
    expect(useStore.getState().addPreview).toBeNull();
  });
});

describe("useDroppedDocuments", () => {
  function Probe() {
    useDroppedDocuments();
    return null;
  }

  async function drop(payload: AddReady) {
    await waitFor(() => expect(mocked.listen.mock.calls.some(([name]) => name === "add_ready")).toBe(true));
    const handler = mocked.listen.mock.calls.find(([name]) => name === "add_ready")![1] as (payload: AddReady) => void;
    act(() => handler(payload));
  }

  it("puts a dropped invoice that was read into the store", async () => {
    const preview = addPreview();
    render(<Probe />);

    await drop({ preview, error: null });

    expect(useStore.getState().addPreview).toEqual(preview);
  });

  it("shows why a dropped file cannot be added as an error toast", async () => {
    render(<Probe />);

    await drop({ preview: null, error: "This PDF has no text" });

    expect(useStore.getState().toasts.map((t) => [t.kind, t.message])).toEqual([["error", "This PDF has no text"]]);
    expect(useStore.getState().addPreview).toBeNull();
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
