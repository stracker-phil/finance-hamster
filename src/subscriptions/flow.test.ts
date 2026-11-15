import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { useStore } from "../state/store";
import { reviewLine } from "../test/review";
import { subscriptionDetail, subscriptionForm } from "../test/subscriptions";
import { changePlan, deleteSubscription, editSubscription, endSubscription, newSubscription, newSubscriptionFrom, resumeSubscription } from "./flow";

const mocked = vi.mocked(api, true);

const toasts = () => useStore.getState().toasts.map((t) => [t.kind, t.message]);
const sheet = () => useStore.getState().subscriptionSheet;

beforeEach(() => {
  vi.clearAllMocks();
  useStore.setState({ toasts: [], subscriptionSheet: null, confirmation: null });
  vi.useFakeTimers({ shouldAdvanceTime: true });
  vi.setSystemTime(new Date(2026, 10, 15, 12));
});

afterEach(() => {
  vi.useRealTimers();
});

describe("newSubscription", () => {
  it("opens the New sheet empty", () => {
    newSubscription();

    expect(sheet()).toEqual({ kind: "new", form: null });
  });
});

describe("newSubscriptionFrom", () => {
  it("opens the New sheet with the form Rust prefilled and the line it came from", async () => {
    const line = reviewLine({ id: "line-9" });
    const form = subscriptionForm({ name: "Example Bookshop", matchText: "EXAMPLEBOOKS" });
    mocked.subscriptionPrefill.mockResolvedValue(form);

    newSubscriptionFrom(line);

    await vi.waitFor(() => expect(sheet()).toEqual({ kind: "new", form, source: line }));
    expect(mocked.subscriptionPrefill).toHaveBeenCalledWith("line-9", null);
  });

  it("asks Rust to match on the selected text when one was given", async () => {
    mocked.subscriptionPrefill.mockResolvedValue(subscriptionForm());

    newSubscriptionFrom(reviewLine({ id: "line-9" }), "CONTRACT 4711");

    await vi.waitFor(() => expect(sheet()).not.toBeNull());
    expect(mocked.subscriptionPrefill).toHaveBeenCalledWith("line-9", "CONTRACT 4711");
  });

  it("shows an error toast and opens no sheet when the line cannot be read", async () => {
    mocked.subscriptionPrefill.mockRejectedValue({ kind: "not_found", message: "Line not found", code: null });

    newSubscriptionFrom(reviewLine());

    await vi.waitFor(() => expect(toasts()).toEqual([["error", "Line not found"]]));
    expect(sheet()).toBeNull();
  });
});

describe("editSubscription", () => {
  it("opens the Edit sheet with the subscription's form", () => {
    const detail = subscriptionDetail({ id: "sub-4", form: subscriptionForm({ name: "Stream Plus" }) });

    editSubscription(detail);

    expect(sheet()).toEqual({ kind: "edit", id: "sub-4", form: detail.form });
  });
});

describe("changePlan", () => {
  it("opens Change Plan starting today with the current plan's amount, currency and interval", () => {
    const detail = subscriptionDetail({
      id: "sub-4",
      name: "Stream Plus",
      currentPlan: "Basic · €8.00 monthly",
      form: subscriptionForm({ amount: "8.00", currency: "EUR", interval: "yearly" }),
    });

    changePlan(detail);

    expect(sheet()).toEqual({
      kind: "plan",
      id: "sub-4",
      name: "Stream Plus",
      currentPlan: "Basic · €8.00 monthly",
      source: undefined,
      form: { name: "", amount: "8.00", currency: "EUR", interval: "yearly", start: "2026-11-15" },
    });
  });

  it("starts from the given charge's amount, currency and date, keeping the interval", () => {
    const detail = subscriptionDetail({ form: subscriptionForm({ amount: "20.00", currency: "EUR", interval: "monthly" }) });

    changePlan(detail, { amount: "25.00", currency: "USD", date: "2026-11-10" });

    expect(sheet()).toMatchObject({ kind: "plan", form: { amount: "25.00", currency: "USD", interval: "monthly", start: "2026-11-10" } });
  });

  it("keeps the bank line it was opened from", () => {
    const line = reviewLine({ id: "line-2" });

    changePlan(subscriptionDetail(), undefined, line);

    expect(sheet()).toMatchObject({ kind: "plan", source: line });
  });
});

describe("endSubscription", () => {
  async function answer(confirmed: boolean, name = "Lumen AI") {
    const done = endSubscription("sub-1", name);
    useStore.getState().confirmation!.answer(confirmed);
    await done;
  }

  it("asks first, naming the subscription", async () => {
    const done = endSubscription("sub-1", "Lumen AI");

    expect(useStore.getState().confirmation).toMatchObject({ title: "End Lumen AI?", action: "End Subscription" });
    useStore.getState().confirmation!.answer(false);
    await done;
  });

  it("ends it today and offers Undo once confirmed", async () => {
    mocked.subscriptionSetEnded.mockResolvedValue("End Lumen AI");

    await answer(true);

    expect(mocked.subscriptionSetEnded).toHaveBeenCalledWith("sub-1", "2026-11-15");
    await vi.waitFor(() => expect(toasts()).toEqual([["success", "End Lumen AI"]]));
    expect(useStore.getState().toasts[0].action?.label).toBe("Undo");
  });

  it("changes nothing when cancelled", async () => {
    await answer(false);

    expect(mocked.subscriptionSetEnded).not.toHaveBeenCalled();
    expect(toasts()).toEqual([]);
  });

  it("shows an error toast when ending fails", async () => {
    mocked.subscriptionSetEnded.mockRejectedValue({ kind: "invalid_input", message: "Tax year 2026 is closed", code: null });

    await answer(true);

    await vi.waitFor(() => expect(toasts()).toEqual([["error", "Tax year 2026 is closed"]]));
  });
});

describe("resumeSubscription", () => {
  it("clears the end date and offers Undo", async () => {
    mocked.subscriptionSetEnded.mockResolvedValue("Resume Lumen AI");

    resumeSubscription("sub-1");

    expect(mocked.subscriptionSetEnded).toHaveBeenCalledWith("sub-1", null);
    await vi.waitFor(() => expect(toasts()).toEqual([["success", "Resume Lumen AI"]]));
  });

  it("shows an error toast when it fails", async () => {
    mocked.subscriptionSetEnded.mockRejectedValue({ kind: "io", message: "Disk full", code: null });

    resumeSubscription("sub-1");

    await vi.waitFor(() => expect(toasts()).toEqual([["error", "Disk full"]]));
  });
});

describe("deleteSubscription", () => {
  function ask(name = "Lumen AI") {
    const result = deleteSubscription("sub-1", name);
    return { result, answer: (confirmed: boolean) => useStore.getState().confirmation!.answer(confirmed) };
  }

  it("asks first, naming the subscription", async () => {
    const { result, answer } = ask();

    expect(useStore.getState().confirmation).toMatchObject({ title: "Delete Lumen AI?", action: "Delete" });
    answer(false);
    await result;
  });

  it("deletes it, offers Undo and returns true once confirmed", async () => {
    mocked.subscriptionDelete.mockResolvedValue("Delete Lumen AI");
    const { result, answer } = ask();

    answer(true);

    expect(await result).toBe(true);
    expect(mocked.subscriptionDelete).toHaveBeenCalledWith("sub-1");
    expect(toasts()).toEqual([["success", "Delete Lumen AI"]]);
  });

  it("returns false and deletes nothing when cancelled", async () => {
    const { result, answer } = ask();

    answer(false);

    expect(await result).toBe(false);
    expect(mocked.subscriptionDelete).not.toHaveBeenCalled();
    expect(toasts()).toEqual([]);
  });

  it("returns false with an error toast when deleting fails", async () => {
    mocked.subscriptionDelete.mockRejectedValue({ kind: "invalid_input", message: "Tax year 2026 is closed", code: null });
    const { result, answer } = ask();

    answer(true);

    expect(await result).toBe(false);
    expect(toasts()).toEqual([["error", "Tax year 2026 is closed"]]);
  });
});
