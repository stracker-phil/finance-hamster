import { fireEvent, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { ComponentProps } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { useStore } from "../state/store";
import { ITEMS, reviewLine } from "../test/review";
import { choose, popUpOptions, popUpValue, withMenus } from "../test/popup";
import { renderWithQuery } from "../test/renderWithQuery";
import { planChanged } from "../test/subscriptions";
import { lineChoices, planChoices, type Runs } from "./choices";
import { ReviewCard } from "./ReviewCard";

const mocked = vi.mocked(api, true);

const LUMEN = { id: "sub-1", name: "Lumen AI", letter: "L", tone: 1 };
const STREAM = { id: "sub-2", name: "Stream Plus", letter: "S", tone: 2 };

const noRuns = (): Runs => ({ decide: vi.fn(), setUp: vi.fn(), updatePlan: vi.fn(), notThis: vi.fn() });

type Props = ComponentProps<typeof ReviewCard>;

function props(patch: Partial<Props> = {}): Props {
  const line = patch.line ?? reviewLine();
  return {
    line,
    choices: lineChoices(line, { itemLabel: undefined, linked: undefined, ruleTerm: null }, noRuns()),
    picked: null,
    items: ITEMS,
    subscriptions: [LUMEN, STREAM],
    item: "",
    itemChosen: false,
    subscription: "",
    always: false,
    term: line.key,
    error: null,
    busy: false,
    itemSelect: { current: null },
    onPick: vi.fn(),
    onConfirm: vi.fn(),
    onItem: vi.fn(),
    onSubscription: vi.fn(),
    onAlways: vi.fn(),
    onTerm: vi.fn(),
    ...patch,
  };
}

function renderCard(patch: Partial<Props> = {}) {
  const p = props(patch);
  renderWithQuery(withMenus(<ReviewCard {...p} />));
  return p;
}

beforeEach(() => {
  vi.clearAllMocks();
  useStore.setState({ menu: null, toasts: [] });
  mocked.rulePreview.mockResolvedValue({ count: 3, text: "3 lines match now", examples: [] });
});

describe("ReviewCard line", () => {
  it("shows the account, date, title, amount and the raw statement text", () => {
    renderCard({ line: reviewLine({ accountName: "Private", date: "Tue, 10 Nov", title: "Example Bookshop", amount: "−€34.90", raw: "Example Bookshop Vienna\nBooking text 4711" }) });

    const card = screen.getByRole("article", { name: "Example Bookshop" });
    expect(within(card).getByText(/Private · Tue, 10 Nov/)).toBeTruthy();
    expect(within(card).getByRole("heading", { name: "Example Bookshop" })).toBeTruthy();
    expect(within(card).getByText("−€34.90")).toBeTruthy();
    expect(within(card).getByText(/Booking text 4711/)).toBeTruthy();
  });

  it("hands over text selected in the line as the term", () => {
    const p = renderCard();
    const raw = screen.getByText(/Booking text 4711/);
    window.getSelection()!.selectAllChildren(raw);

    fireEvent.mouseUp(raw);

    expect(p.onTerm).toHaveBeenCalledWith("Example Bookshop Vienna Booking text 4711");
    window.getSelection()!.removeAllRanges();
  });

  it("ignores a mouse release without a selection", () => {
    const p = renderCard();

    fireEvent.mouseUp(screen.getByText(/Booking text 4711/));

    expect(p.onTerm).not.toHaveBeenCalled();
  });
});

describe("ReviewCard choices", () => {
  it("asks what the line is and offers A, S and I as radios with their hints", () => {
    renderCard();

    expect(screen.getByRole("radiogroup", { name: "What is this line?" })).toBeTruthy();
    for (const [name, hint] of [
      ["ANV (A)", "Counts for the tax year"],
      ["Subscription (S)", "Recurring payment"],
      ["Ignore (I)", "Not relevant"],
    ]) {
      expect(within(screen.getByRole("radio", { name })).getByText(hint)).toBeTruthy();
    }
  });

  it("marks only the picked choice as checked", () => {
    renderCard({ picked: "S" });

    expect(screen.getAllByRole("radio").map((r) => r.getAttribute("aria-checked"))).toEqual(["false", "true", "false"]);
  });

  it("says Suggested, as before on the suggested choice instead of its hint", () => {
    const line = reviewLine({ suggestion: "ignore" });
    renderCard({ line, choices: lineChoices(line, { itemLabel: undefined, linked: undefined, ruleTerm: null }, noRuns()) });

    const ignore = screen.getByRole("radio", { name: "Ignore (I)" });
    expect(within(ignore).getByText("Suggested, as before")).toBeTruthy();
    expect(within(ignore).queryByText("Not relevant")).toBeNull();
  });

  it("reports the key of the clicked choice", async () => {
    const p = renderCard();

    await userEvent.click(screen.getByRole("radio", { name: "Ignore (I)" }));

    expect(p.onPick).toHaveBeenCalledWith("I");
  });
});

describe("ReviewCard confirm", () => {
  it("is disabled as Decide until a choice is picked, and says nothing is decided yet", () => {
    renderCard();

    expect(screen.getByRole("button", { name: /Decide/ }).hasAttribute("disabled")).toBe(true);
    expect(screen.getByText("Nothing is decided until you confirm.")).toBeTruthy();
    expect(screen.getByText("Pick what this line is.")).toBeTruthy();
  });

  it("is named after the picked choice, shows its outcome and confirms on click", async () => {
    const p = renderCard({ picked: "I" });

    expect(screen.getByText("It stays in the statement and needs nothing more.")).toBeTruthy();
    await userEvent.click(screen.getByRole("button", { name: /Ignore Line/ }));

    expect(p.onConfirm).toHaveBeenCalled();
  });

  it("is disabled with the reason when the picked choice is blocked", () => {
    renderCard({ picked: "A" });

    expect(screen.getByText("Choose the item it counts for.")).toBeTruthy();
    expect(screen.getByRole("button", { name: /Count as ANV/ }).hasAttribute("disabled")).toBe(true);
  });

  it("is disabled while the decision is being saved", () => {
    renderCard({ picked: "I", busy: true });

    expect(screen.getByRole("button", { name: /Ignore Line/ }).hasAttribute("disabled")).toBe(true);
  });

  it("shows an error in place of the outcome while keeping the button usable", () => {
    renderCard({ picked: "I", error: "Line was already decided" });

    expect(screen.getByRole("alert").textContent).toBe("Line was already decided");
    expect(screen.queryByText("It stays in the statement and needs nothing more.")).toBeNull();
    expect(screen.getByRole("button", { name: /Ignore Line/ }).hasAttribute("disabled")).toBe(false);
  });

  it("shows the arrow-return key on the confirm button", () => {
    renderCard({ picked: "I" });

    expect(within(screen.getByRole("button", { name: /Ignore Line/ })).getByText("↩")).toBeTruthy();
  });
});

describe("ReviewCard ANV details", () => {
  it("offers a prompt row and every item, showing the current one", async () => {
    renderCard({ picked: "A", item: "kz-720" });

    expect(popUpValue("Counts as")).toBe("KZ 720 · Professional literature");
    expect(await popUpOptions(userEvent, "Counts as")).toEqual(["Choose an item", "KZ 169 · Digital work equipment", "KZ 720 · Professional literature"]);
  });

  it("reports the key of the chosen item", async () => {
    const p = renderCard({ picked: "A" });

    await choose(userEvent, "Counts as", "KZ 169 · Digital work equipment");

    expect(p.onItem).toHaveBeenCalledWith("kz-169");
  });

  it("explains a suggested item until one is chosen", () => {
    renderCard({ line: reviewLine({ itemWhy: "Same as Example Books last month" }), picked: "A" });

    expect(screen.getByText("Same as Example Books last month")).toBeTruthy();
  });

  it("drops the explanation once an item is chosen", () => {
    renderCard({ line: reviewLine({ itemWhy: "Same as Example Books last month" }), picked: "A", item: "kz-720", itemChosen: true });

    expect(screen.queryByText("Same as Example Books last month")).toBeNull();
  });
});

describe("ReviewCard Always option", () => {
  it.each([
    ["ANV", "A"],
    ["Ignore", "I"],
  ])("offers it with the term for %s and reports switching it on", async (_name, key) => {
    const p = renderCard({ picked: key });

    const always = screen.getByRole("checkbox", { name: /Also decide every line containing EXAMPLEBOOKS this way/ });
    expect((always as HTMLInputElement).checked).toBe(false);
    await userEvent.click(always);

    expect(p.onAlways).toHaveBeenCalledWith(true);
  });

  it("shows it checked while on and reports switching it off", async () => {
    const p = renderCard({ picked: "I", always: true });

    const always = screen.getByRole<HTMLInputElement>("checkbox");
    expect(always.checked).toBe(true);
    await userEvent.click(always);

    expect(p.onAlways).toHaveBeenCalledWith(false);
  });

  it("shows the selected text as the term and how many lines it matches", async () => {
    renderCard({ picked: "I", term: "CONTRACT 4711" });

    expect(screen.getByRole("checkbox", { name: /containing CONTRACT 4711 this way/ })).toBeTruthy();
    expect(await screen.findByText("3 lines match now")).toBeTruthy();
    expect(mocked.rulePreview).toHaveBeenCalledWith(expect.objectContaining({ contains: ["CONTRACT 4711"], action: "ignore" }));
  });

  it("explains how to select other text while the term is the line's key", () => {
    renderCard({ picked: "I" });

    expect(screen.getByText("Select text above, such as a contract number, to match on it instead.")).toBeTruthy();
  });

  it("offers the way back to the line's key once other text was selected", async () => {
    const p = renderCard({ picked: "I", term: "CONTRACT 4711" });

    await userEvent.click(screen.getByRole("button", { name: "Use “EXAMPLEBOOKS”" }));

    expect(p.onTerm).toHaveBeenCalledWith(null);
    expect(screen.queryByText(/Select text above/)).toBeNull();
  });

  it.each([
    ["before a choice is picked", null],
    ["for a subscription", "S"],
  ])("is not offered %s", (_name, picked) => {
    renderCard({ picked });

    expect(screen.queryByRole("checkbox")).toBeNull();
  });
});

describe("ReviewCard subscription details", () => {
  it("offers a new subscription first, then every existing one", async () => {
    renderCard({ picked: "S" });

    expect(popUpValue("Subscription")).toBe("A new subscription…");
    expect(await popUpOptions(userEvent, "Subscription")).toEqual(["A new subscription…", "Lumen AI", "Stream Plus"]);
  });

  it("reports the id of the chosen subscription", async () => {
    const p = renderCard({ picked: "S" });

    await choose(userEvent, "Subscription", "Stream Plus");

    expect(p.onSubscription).toHaveBeenCalledWith("sub-2");
  });

  it("says what a new subscription will match on, and lets the owner go back to the key", async () => {
    const p = renderCard({ picked: "S", term: "LUMEN-9" });

    expect(screen.getByText(/It will find its charges by/).textContent).toContain("LUMEN-9");
    await userEvent.click(screen.getByRole("button", { name: "Use “EXAMPLEBOOKS”" }));

    expect(p.onTerm).toHaveBeenCalledWith(null);
  });

  it("shows no matching text when an existing subscription is picked", () => {
    renderCard({ picked: "S", subscription: "sub-1" });

    expect(popUpValue("Subscription")).toBe("Lumen AI");
    expect(screen.queryByText(/It will find its charges by/)).toBeNull();
  });
});

describe("ReviewCard Ignore details", () => {
  it("has nothing more to choose", () => {
    renderCard({ picked: "I" });

    expect(screen.getByText("Nothing more to choose.")).toBeTruthy();
  });
});

describe("ReviewCard for a charge at another amount", () => {
  const plan = planChanged({ name: "Lumen AI", text: "Lumen AI charged €25.00 instead of €20.00." });

  function renderPlanCard(patch: Partial<Props> = {}) {
    const line = reviewLine({ planChanged: plan });
    return renderCard({ line, choices: planChoices(plan, noRuns()), ...patch });
  }

  it("explains the new amount and asks whether the line is that subscription", () => {
    renderPlanCard();

    expect(screen.getByText("Another amount.")).toBeTruthy();
    expect(screen.getByText(/Lumen AI charged €25.00 instead of €20.00\./)).toBeTruthy();
    expect(screen.getByRole("radiogroup", { name: "Is this Lumen AI?" })).toBeTruthy();
  });

  it("offers U, O and N", () => {
    renderPlanCard();

    expect(screen.getAllByRole("radio").map((r) => r.getAttribute("aria-label"))).toEqual(["Yes, new price (U)", "Yes, this once (O)", "No (N)"]);
  });

  it("shows no item, rule or subscription details", () => {
    renderPlanCard({ picked: "U" });

    expect(screen.queryByText("Nothing more to choose.")).toBeNull();
    expect(screen.queryByRole("checkbox")).toBeNull();
    expect(screen.queryByText("Pick what this line is.")).toBeNull();
  });

  it("confirms with the picked choice's button and outcome", async () => {
    const p = renderPlanCard({ picked: "O" });

    expect(screen.getByText("It becomes a charge of Lumen AI; the plan stays as it is.")).toBeTruthy();
    await userEvent.click(screen.getByRole("button", { name: /Link as One-off/ }));

    expect(p.onConfirm).toHaveBeenCalled();
  });

  it("does not look up matches for a rule", async () => {
    renderPlanCard({ picked: "N" });

    await waitFor(() => expect(screen.getByRole("button", { name: /Not This Subscription/ })).toBeTruthy());
    expect(mocked.rulePreview).not.toHaveBeenCalled();
  });
});
