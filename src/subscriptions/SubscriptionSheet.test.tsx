import { act, fireEvent, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api, type SubscriptionForm } from "../api";
import { useStore } from "../state/store";
import { renderWithQuery } from "../test/renderWithQuery";
import { subscriptionForm, subscriptionsView } from "../test/subscriptions";
import { SubscriptionSheet, SubscriptionSheetHost } from "./SubscriptionSheet";

const mocked = vi.mocked(api, true);

/** The empty form's first charge is the real today, read when the module loads; only its shape is pinned. */
const EMPTY_FORM: SubscriptionForm = {
  name: "",
  tone: 0,
  category: "",
  active: true,
  endedOn: null,
  amount: "",
  currency: "EUR",
  interval: "monthly",
  start: expect.stringMatching(/^\d{4}-\d{2}-\d{2}$/),
  reminder: "off",
  itemKey: null,
  matchText: "",
  tolerance: "0",
};

function renderSheet({ id = null, initial = null, onClose = vi.fn() }: { id?: string | null; initial?: SubscriptionForm | null; onClose?: () => void } = {}) {
  return renderWithQuery(<SubscriptionSheet id={id} initial={initial} onClose={onClose} />);
}

const toleranceSelect = () => screen.getByRole<HTMLSelectElement>("combobox", { name: /Compared with the plan/ });
const submit = () => userEvent.click(screen.getByRole("button", { name: "Add Subscription" }));

beforeEach(() => {
  vi.clearAllMocks();
  vi.useFakeTimers({ shouldAdvanceTime: true, toFake: ["Date"] });
  vi.setSystemTime(new Date(2026, 10, 13, 12));
  useStore.setState({ toasts: [], subscriptionSheet: null, confirmation: null });
  mocked.subscriptionsView.mockResolvedValue(subscriptionsView());
  mocked.subscriptionSave.mockResolvedValue("Add Lumen AI");
  mocked.subscriptionMatchCount.mockResolvedValue({ text: "", count: 0 });
});

afterEach(() => {
  vi.useRealTimers();
});

describe("SubscriptionSheet", () => {
  describe("a new subscription", () => {
    it("is titled New Subscription and offers Add Subscription", async () => {
      renderSheet();

      expect(await screen.findByRole("dialog", { name: "New Subscription" })).toBeTruthy();
      expect(screen.getByRole("button", { name: "Add Subscription" })).toBeTruthy();
      expect(screen.queryByRole("button", { name: "Delete Subscription…" })).toBeNull();
    });

    it("sends the typed form with today as the first charge, then closes with an Undo toast", async () => {
      const onClose = vi.fn();
      renderSheet({ onClose });
      await userEvent.type(screen.getByLabelText("Name"), "Lumen AI");
      await userEvent.type(screen.getByLabelText("Amount"), "20.00");

      await submit();

      expect(mocked.subscriptionSave).toHaveBeenCalledWith(null, { ...EMPTY_FORM, name: "Lumen AI", amount: "20.00" });      await waitFor(() => expect(onClose).toHaveBeenCalled());
      expect(useStore.getState().toasts.at(-1)).toMatchObject({ kind: "success", message: "Add Lumen AI", action: { label: "Undo" } });
    });

    it("sends the chosen interval, reminder, category, currency and bank text", async () => {
      renderSheet();
      await userEvent.type(screen.getByLabelText("Name"), "Lumen AI");
      await userEvent.type(screen.getByLabelText("Amount"), "5");

      await userEvent.click(screen.getByRole("button", { name: "Yearly" }));
      await userEvent.selectOptions(screen.getByLabelText("Reminder"), "one_week");
      await userEvent.type(screen.getByLabelText("Category"), "Media");
      await userEvent.clear(screen.getByLabelText("Currency"));
      await userEvent.type(screen.getByLabelText("Currency"), "usd");
      await userEvent.type(screen.getByLabelText("Bank text contains"), "LUMEN");
      await submit();

      expect(mocked.subscriptionSave).toHaveBeenCalledWith(null, { ...EMPTY_FORM, name: "Lumen AI", amount: "5", interval: "yearly", reminder: "one_week", category: "Media", currency: "USD", matchText: "LUMEN" });
    });

    it("marks the chosen repeat interval pressed", async () => {
      renderSheet();

      await userEvent.click(screen.getByRole("button", { name: "Quarterly" }));

      expect(screen.getByRole("button", { name: "Quarterly" }).getAttribute("aria-pressed")).toBe("true");
      expect(screen.getByRole("button", { name: "Monthly" }).getAttribute("aria-pressed")).toBe("false");
    });

    it("calls onClose and saves nothing with Cancel", async () => {
      const onClose = vi.fn();
      renderSheet({ onClose });

      await userEvent.click(screen.getByRole("button", { name: "Cancel" }));

      expect(onClose).toHaveBeenCalled();
      expect(mocked.subscriptionSave).not.toHaveBeenCalled();
    });

    it("starts from a prefilled form", async () => {
      renderSheet({ initial: subscriptionForm({ name: "Example Bookshop", matchText: "EXAMPLEBOOKS", amount: "34.90" }) });

      expect(screen.getByLabelText<HTMLInputElement>("Name").value).toBe("Example Bookshop");
      expect(screen.getByLabelText<HTMLInputElement>("Bank text contains").value).toBe("EXAMPLEBOOKS");
      expect(screen.getByLabelText<HTMLInputElement>("Amount").value).toBe("34.90");
    });

    it("cycles the tile colour with Change color and sends it", async () => {
      renderSheet({ initial: subscriptionForm({ tone: 5, name: "Lumen AI" }) });

      await userEvent.click(screen.getByRole("button", { name: "Change color" }));
      await submit();

      expect(mocked.subscriptionSave).toHaveBeenCalledWith(null, expect.objectContaining({ tone: 0 }));
    });

    it("shows the first letter of the name on the tile", async () => {
      renderSheet();
      expect(screen.getByRole("button", { name: "Change color" }).textContent).toBe("?");

      await userEvent.type(screen.getByLabelText("Name"), "pixel Cloud");

      expect(screen.getByRole("button", { name: "Change color" }).textContent).toBe("P");
    });
  });

  describe("the charge date", () => {
    it("is labelled Next charge for today or later", () => {
      renderSheet({ initial: subscriptionForm({ start: "2026-11-13" }) });

      expect(screen.getByLabelText("Next charge")).toBeTruthy();
      expect(screen.queryByLabelText("First charge")).toBeNull();
    });

    it("is labelled First charge for a date in the past", () => {
      renderSheet({ initial: subscriptionForm({ start: "2026-01-15" }) });

      expect(screen.getByLabelText("First charge")).toBeTruthy();
      expect(screen.queryByLabelText("Next charge")).toBeNull();
    });

    it("sends the date picked", async () => {
      renderSheet({ initial: subscriptionForm() });

      fireEvent.change(screen.getByLabelText("First charge"), { target: { value: "2026-12-01" } });
      await submit();

      expect(mocked.subscriptionSave).toHaveBeenCalledWith(null, expect.objectContaining({ start: "2026-12-01" }));
    });
  });

  describe("ANV", () => {
    it("hides the Item select while the switch is off", () => {
      renderSheet({ initial: subscriptionForm() });

      expect(screen.getByRole<HTMLInputElement>("switch").checked).toBe(false);
      expect(screen.queryByLabelText("Item")).toBeNull();
    });

    it("reveals the Item select when the switch is turned on and sends the first item", async () => {
      renderSheet({ initial: subscriptionForm({ itemKey: "kz-720" }) });
      await screen.findByRole("option", { name: "KZ 169 · Digital work equipment" });
      await userEvent.click(screen.getByRole("switch"));
      expect(screen.queryByLabelText("Item")).toBeNull();

      await userEvent.click(screen.getByRole("switch"));
      await submit();

      expect(screen.getByLabelText("Item")).toBeTruthy();
      expect(mocked.subscriptionSave).toHaveBeenCalledWith(null, expect.objectContaining({ itemKey: "kz-169" }));
    });

    it("sends the item that was chosen", async () => {
      renderSheet({ initial: subscriptionForm({ itemKey: "kz-169" }) });
      await screen.findByRole("option", { name: "KZ 720 · Professional literature" });

      await userEvent.selectOptions(screen.getByLabelText("Item"), "kz-720");
      await submit();

      expect(mocked.subscriptionSave).toHaveBeenCalledWith(null, expect.objectContaining({ itemKey: "kz-720" }));
    });

    it("hides the Item select and sends no item when the switch is turned off", async () => {
      renderSheet({ initial: subscriptionForm({ itemKey: "kz-169" }) });
      expect(screen.getByLabelText("Item")).toBeTruthy();

      await userEvent.click(screen.getByRole("switch"));
      await submit();

      expect(screen.queryByLabelText("Item")).toBeNull();
      expect(mocked.subscriptionSave).toHaveBeenCalledWith(null, expect.objectContaining({ itemKey: null }));
    });
  });

  describe("the amount tolerance", () => {
    it.each([
      ["Exact amount", "0"],
      ["± 0.50", "0.50"],
      ["± 1.00", "1.00"],
      ["± 2.00", "2.00"],
      ["± 5.00", "5.00"],
      ["Any amount", null],
    ])("sends %s as %j", async (label, tolerance) => {
      renderSheet({ initial: subscriptionForm({ tolerance: tolerance === "0" ? "1.00" : "0" }) });

      await userEvent.selectOptions(toleranceSelect(), label);
      await submit();

      expect(mocked.subscriptionSave).toHaveBeenCalledWith(null, expect.objectContaining({ tolerance }));
    });

    it.each([
      ["1,00", "± 1.00"],
      ["0.5", "± 0.50"],
      [null, "Any amount"],
      ["0", "Exact amount"],
    ])("shows a stored tolerance of %j as %s", (stored, label) => {
      renderSheet({ initial: subscriptionForm({ tolerance: stored }) });

      expect(toleranceSelect().selectedOptions[0].textContent).toBe(label);
    });
  });

  describe("the live match text", () => {
    it("shows how many lines the bank text finds", async () => {
      mocked.subscriptionMatchCount.mockResolvedValue({ text: "Finds 3 lines in your statements", count: 3 });

      renderSheet({ initial: subscriptionForm() });

      expect(await screen.findByText("Finds 3 lines in your statements")).toBeTruthy();
    });

    it("asks again for the form as typed", async () => {
      renderSheet({ initial: subscriptionForm({ matchText: "" }) });

      await userEvent.type(screen.getByLabelText("Bank text contains"), "PIXEL");

      await waitFor(() => expect(mocked.subscriptionMatchCount).toHaveBeenCalledWith(expect.objectContaining({ matchText: "PIXEL" })));
    });
  });

  describe("the Cancelled state", () => {
    it("reveals Ended on with today and sends the end date", async () => {
      renderSheet({ initial: subscriptionForm() });
      expect(screen.queryByLabelText("Ended on")).toBeNull();

      await userEvent.click(screen.getByRole("button", { name: "Cancelled" }));

      expect(screen.getByLabelText<HTMLInputElement>("Ended on").value).toBe("2026-11-13");
      await submit();
      expect(mocked.subscriptionSave).toHaveBeenCalledWith(null, expect.objectContaining({ active: false, endedOn: "2026-11-13" }));
    });

    it("hides Ended on and clears the end date when Active is chosen again", async () => {
      renderSheet({ initial: subscriptionForm({ active: false, endedOn: "2026-03-31" }) });
      expect(screen.getByLabelText<HTMLInputElement>("Ended on").value).toBe("2026-03-31");

      await userEvent.click(screen.getByRole("button", { name: "Active" }));
      await submit();

      expect(screen.queryByLabelText("Ended on")).toBeNull();
      expect(mocked.subscriptionSave).toHaveBeenCalledWith(null, expect.objectContaining({ active: true, endedOn: null }));
    });
  });

  describe("when saving fails", () => {
    it("shows a field error under its field and keeps the sheet open", async () => {
      mocked.subscriptionSave.mockRejectedValue({ kind: "invalid_input", message: "Invalid input", code: null, field: { path: "subscription.name", message: "Enter a name." } });
      const onClose = vi.fn();
      renderSheet({ onClose });

      await submit();

      expect((await screen.findByRole("alert")).textContent).toBe("Enter a name.");
      expect(screen.getByLabelText("Name").getAttribute("aria-invalid")).toBe("true");
      expect(screen.getByLabelText("Amount").getAttribute("aria-invalid")).toBe("false");
      expect(screen.getByRole<HTMLButtonElement>("button", { name: "Add Subscription" }).disabled).toBe(false);
      expect(onClose).not.toHaveBeenCalled();
    });

    it.each([
      ["subscription.amount", "Enter an amount.", "Amount"],
      ["subscription.currency", "Unknown currency.", "Currency"],
      ["subscription.match", "Enter a bank text.", "Bank text contains"],
      ["subscription.start", "Choose a date.", "First charge"],
    ])("shows the %s error and marks %s invalid", async (path, message, label) => {
      mocked.subscriptionSave.mockRejectedValue({ kind: "invalid_input", message: "Invalid input", code: null, field: { path, message } });
      renderSheet({ initial: subscriptionForm() });

      await submit();

      expect((await screen.findByRole("alert")).textContent).toBe(message);
      expect(screen.getByLabelText(label).getAttribute("aria-invalid")).toBe("true");
    });

    it("shows an error without a field at the bottom of the sheet", async () => {
      mocked.subscriptionSave.mockRejectedValue({ kind: "io", message: "Disk full", code: null });
      renderSheet({ initial: subscriptionForm() });

      await submit();

      expect((await screen.findByRole("alert")).textContent).toBe("Disk full");
    });
  });

  describe("editing", () => {
    const edit = (onClose = vi.fn()) => renderSheet({ id: "sub-1", initial: subscriptionForm(), onClose });

    it("is titled with the name and offers Save and Delete Subscription…", () => {
      edit();

      expect(screen.getByRole("dialog", { name: "Edit Lumen AI" })).toBeTruthy();
      expect(screen.getByRole("button", { name: "Save" })).toBeTruthy();
      expect(screen.getByRole("button", { name: "Delete Subscription…" })).toBeTruthy();
      expect(screen.queryByRole("button", { name: "Add Subscription" })).toBeNull();
    });

    it("has no charge date field", () => {
      edit();

      expect(screen.queryByLabelText("First charge")).toBeNull();
      expect(screen.queryByLabelText("Next charge")).toBeNull();
    });

    it("saves the edited form for the subscription and closes", async () => {
      const onClose = vi.fn();
      edit(onClose);

      await userEvent.clear(screen.getByLabelText("Name"));
      await userEvent.type(screen.getByLabelText("Name"), "Lumen AI Pro");
      await userEvent.click(screen.getByRole("button", { name: "Save" }));

      expect(mocked.subscriptionSave).toHaveBeenCalledWith("sub-1", subscriptionForm({ name: "Lumen AI Pro" }));
      await waitFor(() => expect(onClose).toHaveBeenCalled());
    });

    it("deletes the subscription after a confirmation and closes", async () => {
      mocked.subscriptionDelete.mockResolvedValue("Delete Lumen AI");
      const onClose = vi.fn();
      edit(onClose);

      await userEvent.click(screen.getByRole("button", { name: "Delete Subscription…" }));
      await waitFor(() => expect(useStore.getState().confirmation?.title).toBe("Delete Lumen AI?"));
      act(() => useStore.getState().confirmation!.answer(true));

      await waitFor(() => expect(onClose).toHaveBeenCalled());
      expect(mocked.subscriptionDelete).toHaveBeenCalledWith("sub-1");
      expect(useStore.getState().toasts.at(-1)?.message).toBe("Delete Lumen AI");
    });

    it("keeps the sheet and the subscription when the deletion is declined", async () => {
      const onClose = vi.fn();
      edit(onClose);

      await userEvent.click(screen.getByRole("button", { name: "Delete Subscription…" }));
      await waitFor(() => expect(useStore.getState().confirmation).not.toBeNull());
      act(() => useStore.getState().confirmation!.answer(false));

      await act(async () => {});
      expect(mocked.subscriptionDelete).not.toHaveBeenCalled();
      expect(onClose).not.toHaveBeenCalled();
    });

    it("keeps the sheet and shows the error when the deletion fails", async () => {
      mocked.subscriptionDelete.mockRejectedValue({ kind: "io", message: "Disk full", code: null });
      const onClose = vi.fn();
      edit(onClose);

      await userEvent.click(screen.getByRole("button", { name: "Delete Subscription…" }));
      await waitFor(() => expect(useStore.getState().confirmation).not.toBeNull());
      act(() => useStore.getState().confirmation!.answer(true));

      await waitFor(() => expect(useStore.getState().toasts.at(-1)).toMatchObject({ kind: "error", message: "Disk full" }));
      expect(onClose).not.toHaveBeenCalled();
    });
  });
});

describe("SubscriptionSheetHost", () => {
  it("shows nothing while no sheet is open", () => {
    renderWithQuery(<SubscriptionSheetHost />);

    expect(screen.queryByRole("dialog")).toBeNull();
  });

  it("shows an empty New sheet for a new subscription", () => {
    useStore.setState({ subscriptionSheet: { kind: "new", form: null } });

    renderWithQuery(<SubscriptionSheetHost />);

    expect(screen.getByRole("dialog", { name: "New Subscription" })).toBeTruthy();
    expect(screen.getByLabelText<HTMLInputElement>("Name").value).toBe("");
  });

  it("shows the New sheet prefilled with the form from the store", () => {
    useStore.setState({ subscriptionSheet: { kind: "new", form: subscriptionForm({ name: "Example Bookshop" }) } });

    renderWithQuery(<SubscriptionSheetHost />);

    expect(screen.getByLabelText<HTMLInputElement>("Name").value).toBe("Example Bookshop");
  });

  it("shows the Edit sheet for an edit", () => {
    useStore.setState({ subscriptionSheet: { kind: "edit", id: "sub-1", form: subscriptionForm() } });

    renderWithQuery(<SubscriptionSheetHost />);

    expect(screen.getByRole("dialog", { name: "Edit Lumen AI" })).toBeTruthy();
  });

  it("shows no subscription sheet for the Change Plan sheet", () => {
    useStore.setState({ subscriptionSheet: { kind: "plan", id: "sub-1", name: "Lumen AI", currentPlan: "Plus", form: { name: "", amount: "25.00", currency: "EUR", interval: "monthly", start: "2026-11-13" } } });

    renderWithQuery(<SubscriptionSheetHost />);

    expect(screen.queryByRole("dialog")).toBeNull();
  });

  it("clears the sheet from the store when it is cancelled", async () => {
    useStore.setState({ subscriptionSheet: { kind: "new", form: null } });
    renderWithQuery(<SubscriptionSheetHost />);

    await userEvent.click(screen.getByRole("button", { name: "Cancel" }));

    expect(useStore.getState().subscriptionSheet).toBeNull();
  });
});
