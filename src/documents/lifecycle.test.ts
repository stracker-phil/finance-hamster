import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { useStore } from "../state/store";
import { runStep, unlock } from "./lifecycle";

const mocked = vi.mocked(api, true);
const toasts = () => useStore.getState().toasts.map((t) => [t.kind, t.message]);

beforeEach(() => {
  vi.clearAllMocks();
  useStore.setState({ toasts: [], confirmation: null });
  mocked.documentStep.mockResolvedValue("Finished Demo Cloud Ltd");
});

describe("runStep", () => {
  it("sends the step and toasts the label it returns", async () => {
    await runStep("doc-1", { kind: "exclude", reason: "Duplicate" });

    expect(mocked.documentStep).toHaveBeenCalledWith("doc-1", { kind: "exclude", reason: "Duplicate" });
    expect(toasts()).toEqual([["success", "Finished Demo Cloud Ltd"]]);
  });

  it("toasts the error message when the step fails", async () => {
    mocked.documentStep.mockRejectedValue({ kind: "invalid_input", message: "Missing a payment date", code: null });

    await runStep("doc-1", { kind: "finish" });

    expect(toasts()).toEqual([["error", "Missing a payment date"]]);
  });
});

describe("unlock", () => {
  it("asks first and unlocks once confirmed", async () => {
    const done = unlock("doc-1");

    expect(useStore.getState().confirmation?.title).toBe("Unlock this document?");
    expect(mocked.documentStep).not.toHaveBeenCalled();
    useStore.getState().confirmation?.answer(true);
    await done;

    expect(mocked.documentStep).toHaveBeenCalledWith("doc-1", { kind: "unlock" });
  });

  it("does nothing when the question is cancelled", async () => {
    const done = unlock("doc-1");

    useStore.getState().confirmation?.answer(false);
    await done;

    expect(mocked.documentStep).not.toHaveBeenCalled();
    expect(toasts()).toEqual([]);
  });
});
