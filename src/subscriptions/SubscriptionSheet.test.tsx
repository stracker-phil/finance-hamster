import { QueryClientProvider } from "@tanstack/react-query";
import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api, type SubscriptionForm } from "../api";
import { createQueryClient } from "../queries/queryClient";
import { SETTINGS_KEY } from "../queries/settings";
import { useStore } from "../state/store";
import { choose, popUpValue, withMenus } from "../test/popup";
import { renderWithQuery } from "../test/renderWithQuery";
import { reviewLine } from "../test/review";
import { settings } from "../test/settings";
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
  reminder: "three_days",
  itemKey: null,
  matchText: "",
  exactOnly: false,
};

function renderSheet({ id = null, initial = null, onClose = vi.fn() }: { id?: string | null; initial?: SubscriptionForm | null; onClose?: () => void } = {}) {
  return renderWithQuery(withMenus(<SubscriptionSheet id={id} initial={initial} onClose={onClose} />));
}

const itemLoaded = (label: string) => waitFor(() => expect(popUpValue("Item")).toBe(label));

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

      expect(mocked.subscriptionSave).toHaveBeenCalledWith(null, { ...EMPTY_FORM, name: "Lumen AI", amount: "20.00" });
      await waitFor(() => expect(onClose).toHaveBeenCalled());
      expect(useStore.getState().toasts.at(-1)).toMatchObject({ kind: "success", message: "Add Lumen AI", action: { label: "Undo" } });
    });

    it("sends the chosen interval, reminder, category, currency and bank text", async () => {
      renderSheet();
      await userEvent.type(screen.getByLabelText("Name"), "Lumen AI");
      await userEvent.type(screen.getByLabelText("Amount"), "5");

      await userEvent.click(screen.getByRole("button", { name: "Yearly" }));
      await choose(userEvent, "Reminder", "1 week before");
      await userEvent.type(screen.getByLabelText("Category"), "Media");
      await userEvent.clear(screen.getByLabelText("Currency"));
      await userEvent.type(screen.getByLabelText("Currency"), "usd");
      await userEvent.type(screen.getByLabelText("Bank text contains"), "LUMEN");
      await submit();

      expect(mocked.subscriptionSave).toHaveBeenCalledWith(null, { ...EMPTY_FORM, name: "Lumen AI", amount: "5", interval: "yearly", reminder: "one_week", category: "Media", currency: "USD", matchText: "LUMEN" });
    });

    it("shows the standard reminder and keeps it when chosen again", async () => {
      renderSheet();
      expect(popUpValue("Reminder")).toBe("3 days before");
      await userEvent.type(screen.getByLabelText("Name"), "Lumen AI");
      await userEvent.type(screen.getByLabelText("Amount"), "20.00");

      await choose(userEvent, "Reminder", "3 days before");
      await submit();

      expect(mocked.subscriptionSave).toHaveBeenCalledWith(null, { ...EMPTY_FORM, name: "Lumen AI", amount: "20.00" });
    });

    it("starts with the default reminder from Settings", async () => {
      const client = createQueryClient();
      client.setQueryData(SETTINGS_KEY, settings({ defaultReminder: "one_week" }));
      render(
        <QueryClientProvider client={client}>
          {withMenus(<SubscriptionSheet id={null} initial={null} onClose={vi.fn()} />)}
        </QueryClientProvider>,
      );
      await waitFor(() => expect(popUpValue("Reminder")).toBe("1 week before"));
      await userEvent.type(screen.getByLabelText("Name"), "Lumen AI");
      await userEvent.type(screen.getByLabelText("Amount"), "20.00");

      await submit();

      expect(mocked.subscriptionSave).toHaveBeenCalledWith(null, { ...EMPTY_FORM, name: "Lumen AI", amount: "20.00", reminder: "one_week" });
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
      await itemLoaded("KZ 720 · Professional literature");
      await userEvent.click(screen.getByRole("switch"));
      expect(screen.queryByLabelText("Item")).toBeNull();

      await userEvent.click(screen.getByRole("switch"));
      await submit();

      expect(screen.getByLabelText("Item")).toBeTruthy();
      expect(mocked.subscriptionSave).toHaveBeenCalledWith(null, expect.objectContaining({ itemKey: "kz-169" }));
    });

    it("sends the item that was chosen", async () => {
      renderSheet({ initial: subscriptionForm({ itemKey: "kz-169" }) });
      await itemLoaded("KZ 169 · Digital work equipment");

      await choose(userEvent, "Item", "KZ 720 · Professional literature");
      await submit();

      expect(popUpValue("Item")).toBe("KZ 720 · Professional literature");
      expect(mocked.subscriptionSave).toHaveBeenCalledWith(null, expect.objectContaining({ itemKey: "kz-720" }));
    });

    it("keeps the item when the current one is chosen again", async () => {
      renderSheet({ initial: subscriptionForm({ itemKey: "kz-169" }) });
      await itemLoaded("KZ 169 · Digital work equipment");

      await choose(userEvent, "Item", "KZ 169 · Digital work equipment");
      await submit();

      expect(mocked.subscriptionSave).toHaveBeenCalledWith(null, expect.objectContaining({ itemKey: "kz-169" }));
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

  describe("another amount", () => {
    const ASK_HINT = "A charge at another amount waits in Review: a new plan, a one-off, or not this subscription.";
    const EXACT_HINT = "Only charges of exactly this amount are this subscription; others are reviewed like any line.";

    it("starts on Ask me with its hint", () => {
      renderSheet({ initial: subscriptionForm() });

      expect(screen.getByRole("button", { name: "Ask me" }).getAttribute("aria-pressed")).toBe("true");
      expect(screen.getByRole("button", { name: "Exact amount only" }).getAttribute("aria-pressed")).toBe("false");
      expect(screen.getByText(ASK_HINT)).toBeTruthy();
    });

    it("sends exactOnly true and shows the other hint after Exact amount only is chosen", async () => {
      renderSheet({ initial: subscriptionForm() });

      await userEvent.click(screen.getByRole("button", { name: "Exact amount only" }));
      await submit();

      expect(screen.getByText(EXACT_HINT)).toBeTruthy();
      expect(screen.queryByText(ASK_HINT)).toBeNull();
      expect(mocked.subscriptionSave).toHaveBeenCalledWith(null, expect.objectContaining({ exactOnly: true }));
    });

    it("shows a stored exact-only subscription on Exact amount only and sends Ask me when switched back", async () => {
      renderSheet({ initial: subscriptionForm({ exactOnly: true }) });
      expect(screen.getByRole("button", { name: "Exact amount only" }).getAttribute("aria-pressed")).toBe("true");

      await userEvent.click(screen.getByRole("button", { name: "Ask me" }));
      await submit();

      expect(mocked.subscriptionSave).toHaveBeenCalledWith(null, expect.objectContaining({ exactOnly: false }));
    });
  });

  describe("the bank line it was opened from", () => {
    const source = reviewLine();

    function renderWithSource() {
      return renderWithQuery(<SubscriptionSheet id={null} initial={subscriptionForm({ matchText: "EXAMPLEBOOKS" })} source={source} onClose={vi.fn()} />);
    }

    function selectInRaw(text: string) {
      const node = screen.getByText(/Booking text 4711/);
      const textNode = node.firstChild!;
      const start = textNode.textContent!.indexOf(text);
      const range = document.createRange();
      range.setStart(textNode, start);
      range.setEnd(textNode, start + text.length);
      const selection = window.getSelection()!;
      selection.removeAllRanges();
      selection.addRange(range);
      fireEvent.mouseUp(node);
    }

    beforeEach(() => {
      window.getSelection()?.removeAllRanges();
    });

    it("shows the line's title, amount, account, date and raw text", () => {
      renderWithSource();

      const line = within(screen.getByLabelText("Bank line"));
      expect(line.getByText("Example Bookshop")).toBeTruthy();
      expect(line.getByText("−€34.90")).toBeTruthy();
      expect(line.getByText("Private · Tue, 10 Nov")).toBeTruthy();
      expect(line.getByText(/Booking text 4711/)).toBeTruthy();
    });

    it("shows no bank line without a source", () => {
      renderSheet({ initial: subscriptionForm() });

      expect(screen.queryByLabelText("Bank line")).toBeNull();
    });

    it("sets the bank text to the selected text, upper-cased, and sends it", async () => {
      renderWithSource();

      selectInRaw("Booking text 4711");
      await submit();

      expect(screen.getByLabelText<HTMLInputElement>(/^Bank text contains/).value).toBe("BOOKING TEXT 4711");
      expect(mocked.subscriptionSave).toHaveBeenCalledWith(null, expect.objectContaining({ matchText: "BOOKING TEXT 4711" }));
    });

    it("keeps the bank text when nothing is selected", () => {
      renderWithSource();

      fireEvent.mouseUp(screen.getByText(/Booking text 4711/));

      expect(screen.getByLabelText<HTMLInputElement>(/^Bank text contains/).value).toBe("EXAMPLEBOOKS");
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

  it("shows the source line of a New sheet opened from Review", () => {
    useStore.setState({ subscriptionSheet: { kind: "new", form: subscriptionForm(), source: reviewLine() } });

    renderWithQuery(<SubscriptionSheetHost />);

    expect(within(screen.getByLabelText("Bank line")).getByText("Example Bookshop")).toBeTruthy();
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
