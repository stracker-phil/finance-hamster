import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { useStore } from "../state/store";
import { showError, undoToast } from "./undoToast";

const mocked = vi.mocked(api, true);

const toasts = () => useStore.getState().toasts.map((t) => [t.kind, t.message]);

beforeEach(() => {
  vi.clearAllMocks();
  useStore.setState({ toasts: [] });
});

describe("showError", () => {
  it("shows the message of an app error as an error toast", () => {
    showError({ kind: "io", message: "Disk full", code: null });

    expect(toasts()).toEqual([["error", "Disk full"]]);
  });

  it("shows a thrown Error's message", () => {
    showError(new Error("Something broke"));

    expect(toasts()).toEqual([["error", "Something broke"]]);
  });

  it("shows a plain string as it is", () => {
    showError("Nope");

    expect(toasts()).toEqual([["error", "Nope"]]);
  });
});

describe("undoToast", () => {
  it("says what changed in a success toast with an Undo action", () => {
    undoToast("Deleted Sample Books");

    expect(toasts()).toEqual([["success", "Deleted Sample Books"]]);
    expect(useStore.getState().toasts[0].action?.label).toBe("Undo");
  });

  it("undoes the change and says what was undone when Undo is run", async () => {
    mocked.undo.mockResolvedValue("Deleted Sample Books");
    undoToast("Deleted Sample Books");

    useStore.getState().toasts[0].action!.run();

    await vi.waitFor(() => expect(toasts()).toContainEqual(["success", "Undid Deleted Sample Books"]));
    expect(mocked.undo).toHaveBeenCalled();
  });

  it("adds no toast when there was nothing to undo", async () => {
    mocked.undo.mockResolvedValue(null);
    undoToast("Deleted Sample Books");

    useStore.getState().toasts[0].action!.run();

    await vi.waitFor(() => expect(mocked.undo).toHaveBeenCalled());
    await Promise.resolve();
    expect(toasts()).toEqual([["success", "Deleted Sample Books"]]);
  });

  it("shows an error toast when undoing fails", async () => {
    mocked.undo.mockRejectedValue({ kind: "invalid_input", message: "Tax year 2026 is closed", code: null });
    undoToast("Deleted Sample Books");

    useStore.getState().toasts[0].action!.run();

    await vi.waitFor(() => expect(toasts()).toContainEqual(["error", "Tax year 2026 is closed"]));
  });
});
