import { act, fireEvent, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { useEvents } from "../queries/useEvents";
import { HOME } from "../state/route";
import { useStore } from "../state/store";
import { choose as choosePopUp, popUpOptions, popUpValue, withMenus } from "../test/popup";
import { renderWithQuery } from "../test/renderWithQuery";
import { reviewLine, reviewQueue } from "../test/review";
import { planChanged, subscriptionDetail, subscriptionForm } from "../test/subscriptions";
import { ReviewPage } from "./ReviewPage";

const mocked = vi.mocked(api, true);

function Page() {
  useEvents();
  return <ReviewPage />;
}

const render = () => renderWithQuery(withMenus(<Page />));

async function emitDataChanged(tables: string[]) {
  await waitFor(() => expect(mocked.listen.mock.calls.some(([name]) => name === "data_changed")).toBe(true));
  const handler = mocked.listen.mock.calls.find(([name]) => name === "data_changed")![1] as (payload: { tables: string[] }) => void;
  await act(async () => handler({ tables }));
}

const itemPicker = () => screen.getByRole("button", { name: "Counts as" });
const noPickers = () => {
  expect(screen.queryByRole("button", { name: "Counts as" })).toBeNull();
  expect(screen.queryByRole("button", { name: "Subscription" })).toBeNull();
};
const radio = (name: string) => screen.getByRole("radio", { name });
const choose = (name: string) => userEvent.click(radio(name));
const confirmButton = (label: string) => screen.getByRole<HTMLButtonElement>("button", { name: (name) => name.startsWith(label) });
const confirm = (label: string) => userEvent.click(confirmButton(label));
const waitForLine = () => screen.findByRole("heading", { name: "Example Bookshop" });

beforeEach(() => {
  vi.clearAllMocks();
  mocked.listen.mockResolvedValue(() => {});
  useStore.setState({ route: { page: "review" }, back: [], forward: [], toasts: [], subscriptionSheet: null });
  mocked.reviewQueue.mockResolvedValue(reviewQueue());
  mocked.reviewDecide.mockResolvedValue("ANV: Example Bookshop");
});

describe("ReviewPage", () => {
  describe("the line card", () => {
    it("shows the line's account, date, title, amount and raw text", async () => {
      render();

      expect(await waitForLine()).toBeTruthy();
      expect(screen.getByText("Private · Tue, 10 Nov")).toBeTruthy();
      expect(screen.getByText("−€34.90")).toBeTruthy();
      expect(screen.getByText(/Booking text 4711/)).toBeTruthy();
    });

    it("offers ANV, Subscription and Ignore with their keys as a radio group, none picked", async () => {
      render();

      expect(await screen.findByRole("radiogroup", { name: "What is this line?" })).toBeTruthy();
      for (const name of ["ANV (A)", "Subscription (S)", "Ignore (I)"]) expect(radio(name).getAttribute("aria-checked")).toBe("false");
    });

    it("reads Decide, disabled, and says nothing is decided while nothing is picked", async () => {
      render();

      await waitForLine();
      expect(confirmButton("Decide").disabled).toBe(true);
      expect(screen.getByText("Nothing is decided until you confirm.")).toBeTruthy();
    });

    it("hints at the keys to pick, confirm and skip", async () => {
      render();

      expect(await screen.findByText(/pick · confirm · skip/)).toBeTruthy();
    });

    it("says a single remaining line needs a decision in the singular", async () => {
      render();

      expect(await screen.findByText("1 line needs your decision")).toBeTruthy();
    });

    it("counts the lines still in the queue", async () => {
      mocked.reviewQueue.mockResolvedValue(reviewQueue({ lines: [reviewLine(), reviewLine({ id: "line-2", title: "Example Cinema" })] }));

      render();

      expect(await screen.findByText("2 lines need your decision")).toBeTruthy();
    });
  });

  describe("the suggestion", () => {
    it("starts with the suggested choice picked and says it repeats an earlier decision", async () => {
      mocked.reviewQueue.mockResolvedValue(reviewQueue({ lines: [reviewLine({ suggestion: "subscription" })] }));

      render();

      await waitForLine();
      expect(radio("Subscription (S)").getAttribute("aria-checked")).toBe("true");
      expect(radio("ANV (A)").getAttribute("aria-checked")).toBe("false");
      expect(radio("Subscription (S)").textContent).toContain("Suggested, as before");
      expect(radio("ANV (A)").textContent).not.toContain("Suggested, as before");
      expect(confirmButton("Set Up Subscription…").disabled).toBe(false);
    });

    it("suggests nothing when the key has no earlier decision", async () => {
      render();

      await waitForLine();
      expect(screen.queryByText("Suggested, as before")).toBeNull();
    });

    it("prefills the item last used for the key and says why", async () => {
      mocked.reviewQueue.mockResolvedValue(reviewQueue({ lines: [reviewLine({ suggestion: "anv", itemKey: "kz-720", itemWhy: "last used for Example Bookshop" })] }));

      render();

      expect(await screen.findByText("last used for Example Bookshop")).toBeTruthy();
      expect(popUpValue("Counts as")).toBe("KZ 720 · Professional literature");
    });

    it("leaves the item unchosen when none was used before", async () => {
      render();
      await waitForLine();

      await choose("ANV (A)");

      expect(popUpValue("Counts as")).toBe("Choose an item");
    });
  });

  describe("picking and confirming", () => {
    it("only picks a choice on click and sends nothing", async () => {
      render();
      await waitForLine();

      await choose("Ignore (I)");

      expect(radio("Ignore (I)").getAttribute("aria-checked")).toBe("true");
      expect(mocked.reviewDecide).not.toHaveBeenCalled();
    });

    it("says what Ignore will do and sends it only on confirm", async () => {
      render();
      await waitForLine();

      await choose("Ignore (I)");
      expect(screen.getByText("It stays in the statement and needs nothing more.")).toBeTruthy();
      await confirm("Ignore Line");

      expect(mocked.reviewDecide).toHaveBeenCalledWith("line-1", "ignore", null, null, false, null);
    });

    it("blocks ANV until an item is chosen", async () => {
      render();
      await waitForLine();

      await choose("ANV (A)");

      expect(confirmButton("Count as ANV").disabled).toBe(true);
      expect(screen.getByText("Choose the item it counts for.")).toBeTruthy();
    });

    it("sends the chosen item with ANV and names it in the outcome", async () => {
      render();
      await waitForLine();

      await choose("ANV (A)");
      await choosePopUp(userEvent, "Counts as", "KZ 169 · Digital work equipment");
      expect(popUpValue("Counts as")).toBe("KZ 169 · Digital work equipment");
      expect(screen.getByText("It counts for KZ 169 · Digital work equipment in its tax year.")).toBeTruthy();
      await confirm("Count as ANV");

      expect(mocked.reviewDecide).toHaveBeenCalledWith("line-1", "anv", "kz-169", null, false, null);
    });

    it("sends the always flag and names the term in the outcome", async () => {
      render();
      await waitForLine();

      await choose("Ignore (I)");
      await userEvent.click(screen.getByRole("checkbox", { name: "Also decide every line containing EXAMPLEBOOKS this way" }));
      expect(screen.getByText("It stays in the statement and needs nothing more, and so will every line containing EXAMPLEBOOKS, also in future statements.")).toBeTruthy();
      await confirm("Ignore Line");

      expect(mocked.reviewDecide).toHaveBeenCalledWith("line-1", "ignore", null, null, true, null);
    });

    it("sends the prefilled item when the owner keeps it", async () => {
      mocked.reviewQueue.mockResolvedValue(reviewQueue({ lines: [reviewLine({ suggestion: "anv", itemKey: "kz-720", itemWhy: "last used for Example Bookshop" })] }));
      render();
      await waitForLine();

      await confirm("Count as ANV");

      expect(mocked.reviewDecide).toHaveBeenCalledWith("line-1", "anv", "kz-720", null, false, null);
    });

    it("keeps the prefilled item when it is chosen again in the picker", async () => {
      mocked.reviewQueue.mockResolvedValue(reviewQueue({ lines: [reviewLine({ suggestion: "anv", itemKey: "kz-720", itemWhy: "last used for Example Bookshop" })] }));
      render();
      await waitForLine();

      await choosePopUp(userEvent, "Counts as", "KZ 720 · Professional literature");

      expect(screen.getByText("last used for Example Bookshop")).toBeTruthy();
      await confirm("Count as ANV");
      expect(mocked.reviewDecide).toHaveBeenCalledWith("line-1", "anv", "kz-720", null, false, null);
    });

    it("shows the item picker only for ANV and the Always checkbox only for ANV and Ignore", async () => {
      render();
      await waitForLine();
      noPickers();
      expect(screen.queryByRole("checkbox")).toBeNull();

      await choose("ANV (A)");
      expect(itemPicker()).toBeTruthy();
      expect(screen.getByRole("checkbox")).toBeTruthy();

      await choose("Ignore (I)");
      expect(screen.queryByRole("button", { name: "Counts as" })).toBeNull();
      expect(screen.getByRole("checkbox")).toBeTruthy();

      await choose("Subscription (S)");
      expect(screen.queryByRole("button", { name: "Counts as" })).toBeNull();
      expect(screen.queryByRole("checkbox")).toBeNull();
    });

    it.each([
      ["a", "ANV (A)"],
      ["s", "Subscription (S)"],
      ["i", "Ignore (I)"],
    ])("picks with the %s key and sends nothing", async (key, name) => {
      render();
      await waitForLine();

      await userEvent.keyboard(key);

      expect(radio(name).getAttribute("aria-checked")).toBe("true");
      expect(mocked.reviewDecide).not.toHaveBeenCalled();
    });

    it("confirms the picked choice with Return", async () => {
      render();
      await waitForLine();

      await userEvent.keyboard("i");
      await userEvent.keyboard("{Enter}");

      expect(mocked.reviewDecide).toHaveBeenCalledWith("line-1", "ignore", null, null, false, null);
    });

    it("sends nothing on Return while nothing is picked or ANV has no item", async () => {
      render();
      await waitForLine();

      await userEvent.keyboard("{Enter}");
      await userEvent.keyboard("a");
      await userEvent.keyboard("{Enter}");

      expect(mocked.reviewDecide).not.toHaveBeenCalled();
    });

    it("toasts the returned label with an Undo that undoes the decision", async () => {
      mocked.undo.mockResolvedValue("ANV: Example Bookshop");
      mocked.reviewQueue.mockResolvedValue(reviewQueue({ lines: [reviewLine({ suggestion: "anv", itemKey: "kz-720" })] }));
      render();
      await waitForLine();

      await confirm("Count as ANV");

      const toast = await waitFor(() => {
        const t = useStore.getState().toasts.at(-1);
        expect(t?.message).toBe("ANV: Example Bookshop");
        return t!;
      });
      expect(toast.kind).toBe("success");
      expect(toast.action?.label).toBe("Undo");
      await act(async () => toast.action!.run());
      await waitFor(() => expect(mocked.undo).toHaveBeenCalled());
      await waitFor(() => expect(useStore.getState().toasts.at(-1)?.message).toBe("Undid ANV: Example Bookshop"));
    });

    it("shows the field message and focuses the item when Rust refuses the item", async () => {
      mocked.reviewDecide.mockRejectedValue({ kind: "invalid_input", message: "Invalid input", code: null, field: { path: "review.item", message: "Choose an item for ANV." } });
      mocked.reviewQueue.mockResolvedValue(reviewQueue({ lines: [reviewLine({ suggestion: "anv", itemKey: "kz-720" })] }));
      render();
      await waitForLine();

      await confirm("Count as ANV");

      expect((await screen.findByRole("alert")).textContent).toBe("Choose an item for ANV.");
      expect(document.activeElement).toBe(itemPicker());
    });

    it("shows the message of any other error and keeps the line", async () => {
      mocked.reviewDecide.mockRejectedValue({ kind: "io", message: "Disk full", code: null });
      render();
      await waitForLine();
      await choose("Ignore (I)");

      await confirm("Ignore Line");

      expect((await screen.findByRole("alert")).textContent).toBe("Disk full");
      expect(screen.getByRole("heading", { name: "Example Bookshop" })).toBeTruthy();
    });
  });

  describe("deciding Subscription", () => {
    const pickSubscription = (name: string) => choosePopUp(userEvent, "Subscription", name);
    const lumen = { id: "sub-1", name: "Lumen AI", letter: "L", tone: 1 };

    beforeEach(() => {
      mocked.reviewQueue.mockResolvedValue(reviewQueue({ subscriptions: [lumen] }));
      mocked.subscriptionPrefill.mockResolvedValue(subscriptionForm({ name: "Example Bookshop", matchText: "EXAMPLEBOOKS" }));
      mocked.reviewDecide.mockResolvedValue("Subscription: Lumen AI");
    });

    it("offers a new subscription by default and the existing ones as choices", async () => {
      render();
      await waitForLine();

      await choose("Subscription (S)");

      expect(popUpValue("Subscription")).toBe("A new subscription…");
      expect(await popUpOptions(userEvent, "Subscription")).toEqual(["A new subscription…", "Lumen AI"]);
      expect(screen.getByText("Opens New Subscription with this line, to check before saving.")).toBeTruthy();
    });

    it("says a new subscription will find its charges by the line's key", async () => {
      render();
      await waitForLine();

      await choose("Subscription (S)");

      expect(screen.getByText(/It will find its charges by/).textContent).toBe("It will find its charges by EXAMPLEBOOKS");
    });

    it("opens the New sheet from the line on confirm, keeping the line as its source", async () => {
      render();
      await waitForLine();
      await choose("Subscription (S)");

      await confirm("Set Up Subscription…");

      await waitFor(() => expect(useStore.getState().subscriptionSheet).toEqual({ kind: "new", form: subscriptionForm({ name: "Example Bookshop", matchText: "EXAMPLEBOOKS" }), source: reviewLine() }));
      expect(mocked.subscriptionPrefill).toHaveBeenCalledWith("line-1", null);
      expect(mocked.reviewDecide).not.toHaveBeenCalled();
    });

    it("opens the New sheet on S then Return", async () => {
      render();
      await waitForLine();

      await userEvent.keyboard("s");
      expect(mocked.subscriptionPrefill).not.toHaveBeenCalled();
      await userEvent.keyboard("{Enter}");

      await waitFor(() => expect(useStore.getState().subscriptionSheet?.kind).toBe("new"));
    });

    it("shows an error toast when the line cannot be prefilled", async () => {
      mocked.subscriptionPrefill.mockRejectedValue({ kind: "not_found", message: "Line not found", code: null });
      render();
      await waitForLine();
      await choose("Subscription (S)");

      await confirm("Set Up Subscription…");

      await waitFor(() => expect(useStore.getState().toasts.at(-1)?.message).toBe("Line not found"));
      expect(useStore.getState().subscriptionSheet).toBeNull();
    });

    it("links the line to the chosen subscription on confirm", async () => {
      render();
      await waitForLine();
      await choose("Subscription (S)");

      await pickSubscription("Lumen AI");
      expect(popUpValue("Subscription")).toBe("Lumen AI");
      expect(screen.getByText("It becomes a charge of Lumen AI.")).toBeTruthy();
      await confirm("Link to Lumen AI");

      expect(mocked.reviewDecide).toHaveBeenCalledWith("line-1", "subscription", null, "sub-1", false, null);
      expect(mocked.subscriptionPrefill).not.toHaveBeenCalled();
    });

    it("links the chosen subscription on Return", async () => {
      render();
      await waitForLine();
      await choose("Subscription (S)");
      await pickSubscription("Lumen AI");
      await userEvent.click(screen.getByRole("heading", { name: "Example Bookshop" }));

      await userEvent.keyboard("{Enter}");

      expect(mocked.reviewDecide).toHaveBeenCalledWith("line-1", "subscription", null, "sub-1", false, null);
    });

    it("sends always as false and counts no new rule, even when Always was checked for ANV", async () => {
      render();
      await waitForLine();
      await choose("ANV (A)");
      await userEvent.click(screen.getByRole("checkbox", { name: "Also decide every line containing EXAMPLEBOOKS this way" }));
      await choose("Subscription (S)");
      await pickSubscription("Lumen AI");

      await confirm("Link to Lumen AI");
      await waitFor(() => expect(mocked.reviewDecide).toHaveBeenCalledWith("line-1", "subscription", null, "sub-1", false, null));
      mocked.reviewQueue.mockResolvedValue(reviewQueue({ lines: [] }));
      await emitDataChanged(["decisions"]);

      expect(await screen.findByText("1 decided · 0 new rules")).toBeTruthy();
    });

    it("sends no subscription with an ANV decision", async () => {
      render();
      await waitForLine();
      await choose("Subscription (S)");
      await pickSubscription("Lumen AI");

      await choose("ANV (A)");
      await choosePopUp(userEvent, "Counts as", "KZ 169 · Digital work equipment");
      await confirm("Count as ANV");

      expect(mocked.reviewDecide).toHaveBeenCalledWith("line-1", "anv", "kz-169", null, false, null);
    });
  });

  describe("the Always term", () => {
    const HINT = "Select text above, such as a contract number, to match on it instead.";
    const alwaysBox = (term: string) => screen.getByRole("checkbox", { name: `Also decide every line containing ${term} this way` });

    function selectText(node: Node, text: string) {
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

    const selectInRaw = (text: string) => selectText(screen.getByText(/Booking text 4711/), text);

    async function renderIgnoring() {
      render();
      await waitForLine();
      await choose("Ignore (I)");
    }

    beforeEach(() => {
      window.getSelection()?.removeAllRanges();
      mocked.rulePreview.mockImplementation((form) => Promise.resolve({ count: 1, text: `Matches ${form.contains.join(" or ")}.`, examples: [] }));
      mocked.subscriptionPrefill.mockResolvedValue(subscriptionForm());
    });

    it("matches the line's key by default and previews what it matches", async () => {
      await renderIgnoring();

      expect(alwaysBox("EXAMPLEBOOKS")).toBeTruthy();
      expect(await screen.findByText("Matches EXAMPLEBOOKS.")).toBeTruthy();
      expect(mocked.rulePreview).toHaveBeenCalledWith(expect.objectContaining({ contains: ["EXAMPLEBOOKS"] }));
      expect(screen.getByText(HINT)).toBeTruthy();
    });

    it("takes the selected text, upper-cased and on one line, as the term", async () => {
      await renderIgnoring();

      selectInRaw("Vienna\nBooking text");

      expect(await screen.findByRole("checkbox", { name: "Also decide every line containing VIENNA BOOKING TEXT this way" })).toBeTruthy();
      expect(await screen.findByText("Matches VIENNA BOOKING TEXT.")).toBeTruthy();
      expect(screen.queryByText(HINT)).toBeNull();
    });

    it("goes back to the key with the Use button", async () => {
      await renderIgnoring();
      selectInRaw("4711");

      await userEvent.click(await screen.findByRole("button", { name: "Use “EXAMPLEBOOKS”" }));

      expect(alwaysBox("EXAMPLEBOOKS")).toBeTruthy();
      expect(screen.getByText(HINT)).toBeTruthy();
      expect(screen.queryByRole("button", { name: /^Use “/ })).toBeNull();
    });

    it("ignores a selection outside the card's text", async () => {
      await renderIgnoring();

      selectText(screen.getByText("What is this line?"), "line");

      expect(alwaysBox("EXAMPLEBOOKS")).toBeTruthy();
      expect(screen.getByText(HINT)).toBeTruthy();
    });

    it("ignores an empty selection", async () => {
      await renderIgnoring();
      selectInRaw("4711");
      await screen.findByRole("button", { name: "Use “EXAMPLEBOOKS”" });

      window.getSelection()!.removeAllRanges();
      fireEvent.mouseUp(screen.getByText(/Booking text 4711/));

      expect(alwaysBox("4711")).toBeTruthy();
    });

    it("keeps the default when the selected text is the key itself", async () => {
      mocked.reviewQueue.mockResolvedValue(reviewQueue({ lines: [reviewLine({ raw: "examplebooks Vienna" })] }));
      await renderIgnoring();

      selectText(screen.getByText("examplebooks Vienna"), "examplebooks");
      await userEvent.click(alwaysBox("EXAMPLEBOOKS"));
      await confirm("Ignore Line");

      expect(screen.getByText(HINT)).toBeTruthy();
      expect(mocked.reviewDecide).toHaveBeenCalledWith("line-1", "ignore", null, null, true, null);
    });

    it("sends the selected term with an Always decision", async () => {
      await renderIgnoring();

      selectInRaw("4711");
      await userEvent.click(await screen.findByRole("checkbox", { name: "Also decide every line containing 4711 this way" }));
      await confirm("Ignore Line");

      expect(mocked.reviewDecide).toHaveBeenCalledWith("line-1", "ignore", null, null, true, "4711");
    });

    it("shows the message of a refused term", async () => {
      mocked.reviewDecide.mockRejectedValue({ kind: "invalid_input", message: "Invalid input", code: null, field: { path: "review.term", message: "That text is too short to match on." } });
      await renderIgnoring();
      selectInRaw("4");

      await confirm("Ignore Line");

      expect((await screen.findByRole("alert")).textContent).toBe("That text is too short to match on.");
    });

    it("prefills a new subscription from the selected term", async () => {
      render();
      await waitForLine();
      await choose("Subscription (S)");
      selectInRaw("4711");
      expect((await screen.findByText(/It will find its charges by/)).textContent).toBe("It will find its charges by 4711");

      await confirm("Set Up Subscription…");

      await waitFor(() => expect(mocked.subscriptionPrefill).toHaveBeenCalledWith("line-1", "4711"));
    });

    it("keeps a term and a pick per line", async () => {
      mocked.reviewQueue.mockResolvedValue(reviewQueue({ lines: [reviewLine(), reviewLine({ id: "line-2", title: "Example Cinema", key: "EXAMPLECINEMA" })] }));
      await renderIgnoring();
      selectInRaw("4711");
      await screen.findByRole("button", { name: "Use “EXAMPLEBOOKS”" });

      await userEvent.click(screen.getByRole("button", { name: "Skip →" }));
      await screen.findByRole("heading", { name: "Example Cinema" });
      expect(radio("Ignore (I)").getAttribute("aria-checked")).toBe("false");
      await choose("Ignore (I)");
      expect(alwaysBox("EXAMPLECINEMA")).toBeTruthy();

      await userEvent.click(screen.getByRole("button", { name: "← Previous" }));
      expect(await screen.findByRole("checkbox", { name: "Also decide every line containing 4711 this way" })).toBeTruthy();
      expect(radio("Ignore (I)").getAttribute("aria-checked")).toBe("true");
    });
  });

  describe("a line whose plan may have changed", () => {
    const line = () => reviewLine({ planChanged: planChanged() });

    beforeEach(() => {
      mocked.reviewQueue.mockResolvedValue(reviewQueue({ lines: [line()], subscriptions: [{ id: "sub-1", name: "Lumen AI", letter: "L", tone: 1 }] }));
      mocked.reviewDecide.mockResolvedValue("Subscription: Lumen AI");
      mocked.subscriptionDetail.mockResolvedValue(subscriptionDetail());
      mocked.subscriptionPrefill.mockResolvedValue(subscriptionForm());
    });

    const waitForPlan = () => screen.findByText("Another amount.");

    it("asks whether the line is the subscription and offers Yes new price, Yes this once and No instead of ANV, Subscription and Ignore", async () => {
      render();

      expect(await waitForPlan()).toBeTruthy();
      expect(screen.getByRole("radiogroup", { name: "Is this Lumen AI?" })).toBeTruthy();
      expect(screen.getByText(/Lumen AI charged €25.00 instead of €20.00\./)).toBeTruthy();
      for (const name of ["Yes, new price (U)", "Yes, this once (O)", "No (N)"]) expect(radio(name)).toBeTruthy();
      for (const name of ["ANV (A)", "Subscription (S)", "Ignore (I)"]) expect(screen.queryByRole("radio", { name })).toBeNull();
    });

    it("starts with nothing picked and Decide disabled", async () => {
      render();

      await waitForPlan();
      for (const name of ["Yes, new price (U)", "Yes, this once (O)", "No (N)"]) expect(radio(name).getAttribute("aria-checked")).toBe("false");
      expect(radio("Yes, new price (U)").textContent).not.toContain("Suggested, as before");
      expect(confirmButton("Decide").disabled).toBe(true);
    });

    it("says what Yes, new price will do once picked", async () => {
      render();
      await waitForPlan();

      await choose("Yes, new price (U)");

      expect(confirmButton("Update Plan…").disabled).toBe(false);
      expect(screen.getByText(/^Opens Change Plan for Lumen AI at 25.00 EUR from this charge/)).toBeTruthy();
    });

    it("hints that No means not this subscription", async () => {
      render();
      await waitForPlan();

      expect(radio("No (N)").textContent).toContain("Not Lumen AI");
    });

    it("hides the item and subscription pickers and the Always checkbox", async () => {
      render();

      await waitForPlan();
      noPickers();
      expect(screen.queryByRole("checkbox")).toBeNull();
    });

    it("links the line to the subscription as a one-off only on confirm", async () => {
      render();
      await waitForPlan();

      await choose("Yes, this once (O)");
      expect(mocked.reviewDecide).not.toHaveBeenCalled();
      await confirm("Link as One-off");

      expect(mocked.reviewDecide).toHaveBeenCalledWith("line-1", "subscription", null, "sub-1", false, null);
    });

    it("opens Change Plan with the amount, currency and date of the charge, and the line as its source", async () => {
      render();
      await waitForPlan();
      await choose("Yes, new price (U)");

      await confirm("Update Plan…");

      await waitFor(() => expect(useStore.getState().subscriptionSheet).not.toBeNull());
      expect(mocked.subscriptionDetail).toHaveBeenCalledWith("sub-1");
      expect(useStore.getState().subscriptionSheet).toEqual({
        kind: "plan",
        id: "sub-1",
        name: "Lumen AI",
        currentPlan: "Plus · €20.00 monthly",
        source: line(),
        form: { name: "", amount: "25.00", currency: "EUR", interval: "monthly", start: "2026-11-10" },
      });
    });

    it("shows the error when the subscription cannot be loaded for Update Plan", async () => {
      mocked.subscriptionDetail.mockRejectedValue({ kind: "not_found", message: "Subscription not found", code: null });
      render();
      await waitForPlan();
      await choose("Yes, new price (U)");

      await confirm("Update Plan…");

      expect((await screen.findByRole("alert")).textContent).toBe("Subscription not found");
    });

    it("opens Change Plan on U then Return", async () => {
      render();
      await waitForPlan();

      await userEvent.keyboard("u");
      await userEvent.keyboard("{Enter}");

      await waitFor(() => expect(useStore.getState().subscriptionSheet?.kind).toBe("plan"));
    });

    it("does nothing on Return while nothing is picked", async () => {
      render();
      await waitForPlan();

      await userEvent.keyboard("{Enter}");

      expect(mocked.reviewDecide).not.toHaveBeenCalled();
      expect(useStore.getState().subscriptionSheet).toBeNull();
    });

    it("decides a one-off on O then Return", async () => {
      render();
      await waitForPlan();

      await userEvent.keyboard("o");
      expect(mocked.reviewDecide).not.toHaveBeenCalled();
      await userEvent.keyboard("{Enter}");

      expect(mocked.reviewDecide).toHaveBeenCalledWith("line-1", "subscription", null, "sub-1", false, null);
    });

    it("ignores the A, S and I keys", async () => {
      render();
      await waitForPlan();

      await userEvent.keyboard("asi");

      for (const name of ["Yes, new price (U)", "Yes, this once (O)", "No (N)"]) expect(radio(name).getAttribute("aria-checked")).toBe("false");
      expect(mocked.reviewDecide).not.toHaveBeenCalled();
      expect(mocked.subscriptionPrefill).not.toHaveBeenCalled();
    });

    describe("answered No", () => {
      beforeEach(() => {
        mocked.reviewQueue.mockResolvedValue(
          reviewQueue({ lines: [reviewLine({ planChanged: planChanged(), suggestion: "anv" })], subscriptions: [{ id: "sub-1", name: "Lumen AI", letter: "L", tone: 1 }] }),
        );
      });

      it("sends nothing and asks What is this line? with ANV, Subscription and Ignore", async () => {
        render();
        await waitForPlan();

        await choose("No (N)");
        await confirm("Not This Subscription");

        expect(await screen.findByRole("radiogroup", { name: "What is this line?" })).toBeTruthy();
        for (const name of ["ANV (A)", "Subscription (S)", "Ignore (I)"]) expect(radio(name)).toBeTruthy();
        expect(screen.queryByRole("radio", { name: "No (N)" })).toBeNull();
        expect(mocked.reviewDecide).not.toHaveBeenCalled();
        expect(mocked.subscriptionPrefill).not.toHaveBeenCalled();
      });

      it("picks nothing but the line's suggestion and shows the details of the ordinary line", async () => {
        render();
        await waitForPlan();

        await userEvent.keyboard("n");
        await userEvent.keyboard("{Enter}");

        await screen.findByRole("radiogroup", { name: "What is this line?" });
        expect(radio("ANV (A)").getAttribute("aria-checked")).toBe("true");
        expect(radio("Subscription (S)").getAttribute("aria-checked")).toBe("false");
        expect(radio("Ignore (I)").getAttribute("aria-checked")).toBe("false");
        expect(itemPicker()).toBeTruthy();
      });

      it("opens the New sheet from the line with Subscription", async () => {
        render();
        await waitForPlan();
        await choose("No (N)");
        await confirm("Not This Subscription");

        await choose("Subscription (S)");
        await confirm("Set Up Subscription…");

        await waitFor(() => expect(useStore.getState().subscriptionSheet?.kind).toBe("new"));
        expect(mocked.subscriptionPrefill).toHaveBeenCalledWith("line-1", null);
      });
    });
  });

  describe("moving between lines", () => {
    beforeEach(() => {
      mocked.reviewQueue.mockResolvedValue(reviewQueue({ lines: [reviewLine(), reviewLine({ id: "line-2", title: "Example Cinema" })] }));
    });

    it("disables Previous on the first line", async () => {
      render();

      const previous = await screen.findByRole<HTMLButtonElement>("button", { name: "← Previous" });
      expect(previous.disabled).toBe(true);
    });

    it("skips to the next line and back to the previous one", async () => {
      render();

      await userEvent.click(await screen.findByRole("button", { name: "Skip →" }));
      expect(await screen.findByRole("heading", { name: "Example Cinema" })).toBeTruthy();
      expect(screen.getByText("2 lines need your decision")).toBeTruthy();

      await userEvent.click(screen.getByRole("button", { name: "← Previous" }));
      expect(await screen.findByRole("heading", { name: "Example Bookshop" })).toBeTruthy();
    });

    it("skips with the J key", async () => {
      render();
      await waitForLine();

      await userEvent.keyboard("j");

      expect(await screen.findByRole("heading", { name: "Example Cinema" })).toBeTruthy();
    });

    it("decides the line that is showing after a skip", async () => {
      render();
      await userEvent.click(await screen.findByRole("button", { name: "Skip →" }));
      await screen.findByRole("heading", { name: "Example Cinema" });

      await choose("Ignore (I)");
      await confirm("Ignore Line");

      expect(mocked.reviewDecide).toHaveBeenCalledWith("line-2", "ignore", null, null, false, null);
    });

    it("disables Skip when only one line is left", async () => {
      mocked.reviewQueue.mockResolvedValue(reviewQueue());

      render();

      const skip = await screen.findByRole<HTMLButtonElement>("button", { name: "Skip →" });
      expect(skip.disabled).toBe(true);
    });
  });

  describe("when no line is left", () => {
    beforeEach(() => {
      mocked.reviewQueue.mockResolvedValue(reviewQueue({ lines: [] }));
    });

    it("says that no line is waiting when the queue starts empty", async () => {
      render();

      expect(await screen.findByText("All reviewed")).toBeTruthy();
      expect(screen.getByText("No line is waiting for a decision.")).toBeTruthy();
    });

    it("counts the decisions and new rules made in this session", async () => {
      mocked.reviewQueue.mockResolvedValue(reviewQueue());
      render();
      await waitForLine();
      await choose("Ignore (I)");
      await userEvent.click(screen.getByRole("checkbox", { name: "Also decide every line containing EXAMPLEBOOKS this way" }));
      await confirm("Ignore Line");
      mocked.reviewQueue.mockResolvedValue(reviewQueue({ lines: [] }));

      await emitDataChanged(["decisions"]);

      expect(await screen.findByText("1 decided · 1 new rule")).toBeTruthy();
    });

    it("counts a decision without a rule as no new rules", async () => {
      mocked.reviewQueue.mockResolvedValue(reviewQueue());
      render();
      await waitForLine();
      await choose("Ignore (I)");
      await confirm("Ignore Line");
      mocked.reviewQueue.mockResolvedValue(reviewQueue({ lines: [] }));

      await emitDataChanged(["decisions"]);

      expect(await screen.findByText("1 decided · 0 new rules")).toBeTruthy();
    });

    it("goes back to Statements", async () => {
      render();

      await userEvent.click(await screen.findByRole("button", { name: "Back to Statements" }));

      expect(useStore.getState().route).toEqual({ page: "statements" });
    });

    it("goes back to Today", async () => {
      useStore.setState({ route: HOME });
      render();

      await userEvent.click(await screen.findByRole("button", { name: "Back to Today" }));

      expect(useStore.getState().route).toEqual({ page: "today" });
    });
  });
});
