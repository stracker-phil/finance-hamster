import { describe, expect, it, vi } from "vitest";
import { planChanged } from "../test/subscriptions";
import { reviewLine } from "../test/review";
import { lineChoices, planChoices, type Runs } from "./choices";

function runs(): Runs {
  return { decide: vi.fn(), setUp: vi.fn(), updatePlan: vi.fn(), notThis: vi.fn() };
}

const NO_PICKS = { itemLabel: undefined, linked: undefined, ruleTerm: null };

describe("lineChoices", () => {
  it("offers ANV, Subscription and Ignore under the keys A, S and I", () => {
    const choices = lineChoices(reviewLine(), NO_PICKS, runs());

    expect(choices.map((c) => [c.key, c.label, c.action])).toEqual([
      ["A", "ANV", "anv"],
      ["S", "Subscription", "subscription"],
      ["I", "Ignore", "ignore"],
    ]);
  });

  it("marks only the suggested action as suggested", () => {
    const choices = lineChoices(reviewLine({ suggestion: "subscription" }), NO_PICKS, runs());

    expect(choices.map((c) => !!c.suggested)).toEqual([false, true, false]);
  });

  it("suggests nothing for a line without a suggestion", () => {
    const choices = lineChoices(reviewLine({ suggestion: null }), NO_PICKS, runs());

    expect(choices.some((c) => c.suggested)).toBe(false);
  });

  describe("ANV", () => {
    it("is blocked until an item is chosen", () => {
      const [anv] = lineChoices(reviewLine(), NO_PICKS, runs());

      expect(anv.blocked).toBe("Choose the item it counts for.");
      expect(anv.confirm).toBe("Count as ANV");
    });

    it("names the chosen item in its outcome and is no longer blocked", () => {
      const [anv] = lineChoices(reviewLine(), { ...NO_PICKS, itemLabel: "KZ 720 · Professional literature" }, runs());

      expect(anv.blocked).toBeUndefined();
      expect(anv.outcome).toBe("It counts for KZ 720 · Professional literature in its tax year.");
    });

    it("decides ANV without a subscription when run", () => {
      const r = runs();
      const [anv] = lineChoices(reviewLine(), { ...NO_PICKS, itemLabel: "KZ 720" }, r);

      anv.run();

      expect(r.decide).toHaveBeenCalledWith("anv", null);
    });

    it("tells that the Always rule covers later statements too", () => {
      const [anv] = lineChoices(reviewLine(), { ...NO_PICKS, itemLabel: "KZ 720", ruleTerm: "EXAMPLEBOOKS" }, runs());

      expect(anv.outcome).toBe("It counts for KZ 720 in its tax year, and so will every line containing EXAMPLEBOOKS, also in future statements.");
    });
  });

  describe("Subscription", () => {
    it("opens New Subscription from the line when no existing one is picked", () => {
      const r = runs();
      const [, subscription] = lineChoices(reviewLine(), NO_PICKS, r);

      subscription.run();

      expect(subscription.confirm).toBe("Set Up Subscription…");
      expect(r.setUp).toHaveBeenCalled();
      expect(r.decide).not.toHaveBeenCalled();
    });

    it("links to the picked subscription by id", () => {
      const r = runs();
      const [, subscription] = lineChoices(reviewLine(), { ...NO_PICKS, linked: { id: "sub-7", name: "Lumen AI", letter: "L", tone: 1 } }, r);

      subscription.run();

      expect(subscription.confirm).toBe("Link to Lumen AI");
      expect(subscription.outcome).toBe("It becomes a charge of Lumen AI.");
      expect(r.decide).toHaveBeenCalledWith("subscription", "sub-7");
    });
  });

  describe("Ignore", () => {
    it("decides Ignore when run", () => {
      const r = runs();
      const [, , ignore] = lineChoices(reviewLine(), NO_PICKS, r);

      ignore.run();

      expect(ignore.confirm).toBe("Ignore Line");
      expect(r.decide).toHaveBeenCalledWith("ignore", null);
    });

    it("mentions the Always rule's term in its outcome", () => {
      const [, , ignore] = lineChoices(reviewLine(), { ...NO_PICKS, ruleTerm: "EXAMPLEBOOKS" }, runs());

      expect(ignore.outcome).toBe("It stays in the statement and needs nothing more, and so will every line containing EXAMPLEBOOKS, also in future statements.");
    });

    it("says nothing about a rule when Always is off", () => {
      const [, , ignore] = lineChoices(reviewLine(), NO_PICKS, runs());

      expect(ignore.outcome).toBe("It stays in the statement and needs nothing more.");
    });
  });
});

describe("planChoices", () => {
  it("offers a new price, this once and No under the keys U, O and N, none with an action", () => {
    const choices = planChoices(planChanged(), runs());

    expect(choices.map((c) => [c.key, c.label, c.action])).toEqual([
      ["U", "Yes, new price", undefined],
      ["O", "Yes, this once", undefined],
      ["N", "No", undefined],
    ]);
  });

  it("names the plan, the new amount and the currency in the Update Plan outcome", () => {
    const [update] = planChoices(planChanged({ name: "Lumen AI", amount: "25.00", currency: "EUR" }), runs());

    expect(update.confirm).toBe("Update Plan…");
    expect(update.outcome).toContain("Change Plan for Lumen AI at 25.00 EUR");
  });

  it("opens Change Plan on U, links as a one-off to the subscription on O and asks the line normally on N", () => {
    const r = runs();
    const [update, once, no] = planChoices(planChanged({ subscriptionId: "sub-3" }), r);

    update.run();
    expect(r.updatePlan).toHaveBeenCalled();
    expect(r.decide).not.toHaveBeenCalled();

    once.run();
    expect(r.decide).toHaveBeenCalledWith("subscription", "sub-3");

    no.run();
    expect(r.notThis).toHaveBeenCalled();
  });

  it("names the subscription in the hint of No", () => {
    const [, , no] = planChoices(planChanged({ name: "Lumen AI" }), runs());

    expect(no.hint).toBe("Not Lumen AI");
    expect(no.confirm).toBe("Not This Subscription");
  });
});
