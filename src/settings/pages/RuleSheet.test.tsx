import { act, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../../test/apiMock")).createApiMock() }));

import { api } from "../../api";
import { ConfirmHost } from "../../components/ConfirmSheet";
import { useStore } from "../../state/store";
import { account } from "../../test/accounts";
import { ITEMS, ruleForm, ruleView } from "../../test/lineRules";
import { choose, popUpValue, withMenus } from "../../test/popup";
import { renderWithQuery } from "../../test/renderWithQuery";
import { RuleSheet } from "./RuleSheet";

const mocked = vi.mocked(api, true);
const messages = () => useStore.getState().toasts.map((t) => t.message);
const onClose = vi.fn();

const renderSheet = (rule?: ReturnType<typeof ruleView>) =>
  renderWithQuery(
    withMenus(
      <>
        <RuleSheet rule={rule} items={ITEMS} onClose={onClose} />
        <ConfirmHost />
      </>,
    ),
  );

/** The account list is a read; its menu is built from what has arrived when it opens. */
const accountsLoaded = () => act(async () => void (await mocked.listAccounts.mock.results[0]?.value));

const contains = () => screen.getByLabelText<HTMLInputElement>("Bank text contains");

beforeEach(() => {
  vi.clearAllMocks();
  mocked.rulePreview.mockReset();
  mocked.rulePreview.mockResolvedValue({ count: 0, text: "", examples: [] });
  useStore.setState({ toasts: [], confirmation: null });
  mocked.listAccounts.mockResolvedValue([account(), account({ id: "acc-2", name: "Savings" })]);
});

describe("RuleSheet", () => {
  describe("new rule", () => {
    it("opens empty with Ignore chosen and no item picker", () => {
      renderSheet();

      expect(screen.getByRole("dialog", { name: "New Rule" })).toBeTruthy();
      expect(contains().value).toBe("");
      expect(screen.getByRole("button", { name: "Ignore" }).getAttribute("aria-pressed")).toBe("true");
      expect(screen.queryByLabelText("Item")).toBeNull();
      expect(screen.queryByRole("button", { name: "Delete…" })).toBeNull();
    });

    it("saves the bank text split on commas as typed, with an empty amount as null, then toasts and closes", async () => {
      renderSheet();
      await userEvent.type(contains(), " ALPENMARKT , billa,");

      await userEvent.click(screen.getByRole("button", { name: "Add Rule" }));

      expect(mocked.ruleSave).toHaveBeenCalledWith(null, ruleForm({ contains: [" ALPENMARKT ", " billa", ""] }));
      await vi.waitFor(() => expect(onClose).toHaveBeenCalled());
      expect(messages()).toEqual(["Added the rule"]);
    });

    it("saves an assign-item rule with the amount, tolerance, account and own-counterparty flag", async () => {
      renderSheet();
      await userEvent.type(contains(), "CLOUDBOX");
      await accountsLoaded();
      await userEvent.type(screen.getByLabelText("Amount in €"), "25.95");
      await userEvent.type(screen.getByLabelText("Within ± €"), "0.50");
      await choose(userEvent, "Account", "Savings");
      await userEvent.click(screen.getByRole("switch", { name: "Counterparty is one of my accounts" }));
      await userEvent.click(screen.getByRole("button", { name: "Assign item" }));
      await choose(userEvent, "Item", "KZ 169 · Digitale Arbeitsmittel");

      await userEvent.click(screen.getByRole("button", { name: "Add Rule" }));

      expect(mocked.ruleSave).toHaveBeenCalledWith(null, {
        contains: ["CLOUDBOX"],
        amount: "25.95",
        tolerance: "0.50",
        accountId: "acc-2",
        ownCounterparty: true,
        action: "anv",
        itemKey: "tools",
      });
    });

    it("shows the item picker only while Assign item is chosen", async () => {
      renderSheet();

      await userEvent.click(screen.getByRole("button", { name: "Assign item" }));
      expect(screen.getByLabelText("Item")).toBeTruthy();

      await userEvent.click(screen.getByRole("button", { name: "Ignore" }));
      expect(screen.queryByLabelText("Item")).toBeNull();
    });
  });

  describe("edit rule", () => {
    const stored = ruleView({
      form: ruleForm({ contains: ["BILLA", "SPAR"], amount: "12.00", tolerance: "1.00", accountId: "acc-2", ownCounterparty: true, action: "anv", itemKey: "books" }),
    });

    it("opens prefilled from the rule", async () => {
      renderSheet(stored);

      expect(screen.getByRole("dialog", { name: "Edit Rule" })).toBeTruthy();
      expect(contains().value).toBe("BILLA, SPAR");
      expect(screen.getByLabelText<HTMLInputElement>("Amount in €").value).toBe("12.00");
      expect(screen.getByLabelText<HTMLInputElement>("Within ± €").value).toBe("1.00");
      await waitFor(() => expect(popUpValue("Account")).toBe("Savings"));
      expect(screen.getByRole("switch", { name: "Counterparty is one of my accounts" })).toBeChecked();
      expect(screen.getByRole("button", { name: "Assign item" }).getAttribute("aria-pressed")).toBe("true");
      expect(popUpValue("Item")).toBe("KZ 720 · Fachliteratur");
    });

    it("saves nothing new when the current account and item are chosen again", async () => {
      renderSheet(stored);
      await waitFor(() => expect(popUpValue("Account")).toBe("Savings"));

      await choose(userEvent, "Account", "Savings");
      await choose(userEvent, "Item", "KZ 720 · Fachliteratur");
      await userEvent.click(screen.getByRole("button", { name: "Save" }));

      expect(mocked.ruleSave).toHaveBeenCalledWith("rule-1", expect.objectContaining({ accountId: "acc-2", itemKey: "books" }));
    });

    it("saves under the rule's id, then toasts Saved and closes", async () => {
      renderSheet(stored);
      await userEvent.clear(contains());
      await userEvent.type(contains(), "COOP");

      await userEvent.click(screen.getByRole("button", { name: "Save" }));

      expect(mocked.ruleSave).toHaveBeenCalledWith("rule-1", { ...stored.form, contains: ["COOP"] });
      await vi.waitFor(() => expect(onClose).toHaveBeenCalled());
      expect(messages()).toEqual(["Saved the rule"]);
    });

    it("offers Delete… for a rule the user made but not for a built-in one", () => {
      const { unmount } = renderSheet(ruleView());
      expect(screen.getByRole("button", { name: "Delete…" })).toBeTruthy();
      unmount();

      renderSheet(ruleView({ builtin: true }));
      expect(screen.queryByRole("button", { name: "Delete…" })).toBeNull();
    });
  });

  describe("delete", () => {
    it("asks first, then deletes the rule, toasts and closes", async () => {
      renderSheet(ruleView());
      await userEvent.click(screen.getByRole("button", { name: "Delete…" }));
      expect(mocked.ruleDelete).not.toHaveBeenCalled();
      expect(screen.getByRole("alertdialog", { name: "Delete this rule?" })).toBeTruthy();

      await userEvent.click(within(screen.getByRole("alertdialog")).getByRole("button", { name: "Delete" }));

      await vi.waitFor(() => expect(mocked.ruleDelete).toHaveBeenCalledWith("rule-1"));
      await vi.waitFor(() => expect(onClose).toHaveBeenCalled());
      expect(messages()).toEqual(["Deleted the rule"]);
    });

    it("keeps the rule when the question is cancelled", async () => {
      renderSheet(ruleView());
      await userEvent.click(screen.getByRole("button", { name: "Delete…" }));

      await userEvent.click(within(screen.getByRole("alertdialog")).getByRole("button", { name: "Cancel" }));

      expect(mocked.ruleDelete).not.toHaveBeenCalled();
      expect(onClose).not.toHaveBeenCalled();
      expect(screen.getByRole("dialog", { name: "Edit Rule" })).toBeTruthy();
    });
  });

  describe("help and preview", () => {
    it("explains how the conditions combine and how subscriptions are matched", () => {
      renderSheet();

      expect(screen.getByText(/Every condition you fill in must hold/)).toBeTruthy();
      expect(screen.getByText(/A subscription finds its own charges by its bank text and plan amount/)).toBeTruthy();
    });

    it("shows what the rule would match, with example lines", async () => {
      mocked.rulePreview.mockResolvedValue({ count: 5, text: "Matches 5 lines", examples: ["10 Nov · Example Market · −€22.48", "3 Nov · Example Market · −€8.10", "28 Oct · Example Market · −€15.00"] });
      renderSheet();

      expect(await screen.findByText("Matches 5 lines")).toBeTruthy();
      expect(screen.getByText("10 Nov · Example Market · −€22.48")).toBeTruthy();
      expect(screen.getByText("3 Nov · Example Market · −€8.10")).toBeTruthy();
      expect(screen.getByText("28 Oct · Example Market · −€15.00")).toBeTruthy();
    });

    it("asks for the preview of the draft with the bank text split on commas", async () => {
      renderSheet();

      await userEvent.type(contains(), "BILLA,SPAR");

      await vi.waitFor(() => expect(mocked.rulePreview).toHaveBeenLastCalledWith(ruleForm({ contains: ["BILLA", "SPAR"] })));
    });

    it("updates the preview when the draft changes", async () => {
      mocked.rulePreview.mockImplementation((form) => Promise.resolve({ count: 0, text: `Looks for ${form.contains.join(" or ")}`, examples: [] }));
      renderSheet();

      await userEvent.type(contains(), "COOP");

      expect(await screen.findByText("Looks for COOP")).toBeTruthy();
    });
  });

  describe("errors", () => {
    it("shows a field error under its field and keeps the sheet open", async () => {
      mocked.ruleSave.mockRejectedValue({ kind: "invalid_input", message: "`rule.conditions`: Add a condition", field: { path: "rule.conditions", message: "Add a condition" } });
      renderSheet();

      await userEvent.click(screen.getByRole("button", { name: "Add Rule" }));

      expect((await screen.findByRole("alert")).textContent).toBe("Add a condition");
      expect(onClose).not.toHaveBeenCalled();
      expect(messages()).toEqual([]);
      expect(screen.getByRole("button", { name: "Add Rule" })).toBeEnabled();
    });

    it("shows an error without a field at the bottom of the form", async () => {
      mocked.ruleSave.mockRejectedValue({ kind: "io", message: "Disk full" });
      renderSheet();

      await userEvent.click(screen.getByRole("button", { name: "Add Rule" }));

      expect((await screen.findByRole("alert")).textContent).toBe("Disk full");
      expect(onClose).not.toHaveBeenCalled();
    });
  });
});
