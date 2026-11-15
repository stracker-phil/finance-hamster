import type { Action, PlanChanged, ReviewLine, SubscriptionOption } from "../api";
import { t } from "../i18n";

/** One answer to "What is this line?": its key picks it, the confirm button runs it. */
export interface Choice {
  key: string;
  label: string;
  hint: string;
  /** Absent for a "Plan changed?" line's choices. */
  action?: Action;
  suggested?: boolean;
  confirm: string;
  /** What confirming does, in one sentence. */
  outcome: string;
  /** Why confirming isn't possible yet. */
  blocked?: string;
  run: () => void;
}

const KEYS: Record<Action, string> = { anv: "A", subscription: "S", ignore: "I" };

/** What confirming runs; the page owns the writes. */
export interface Runs {
  decide: (action: Action, subscriptionId: string | null) => void;
  setUp: () => void;
  updatePlan: () => void;
  /** "No, not this subscription": the line is asked like any other. */
  notThis: () => void;
}

/** "Is this <subscription>?" for a charge at another amount than its plan. */
export function planChoices(plan: PlanChanged, runs: Runs): Choice[] {
  const text = t().review.choices;
  return [
    {
      key: "U",
      label: text.newPrice,
      hint: text.newPriceHint,
      confirm: text.updatePlan,
      outcome: text.updatePlanOutcome(plan.name, plan.amount, plan.currency),
      run: runs.updatePlan,
    },
    {
      key: "O",
      label: text.once,
      hint: text.onceHint,
      confirm: text.oneOff,
      outcome: text.oneOffOutcome(plan.name),
      run: () => runs.decide("subscription", plan.subscriptionId),
    },
    {
      key: "N",
      label: text.no,
      hint: text.notName(plan.name),
      confirm: text.notThis,
      outcome: text.notThisOutcome,
      run: runs.notThis,
    },
  ];
}

interface Picks {
  /** The label of the item an ANV decision counts for; absent until one is chosen. */
  itemLabel: string | undefined;
  /** The existing subscription picked for a Subscription decision; absent for a new one. */
  linked: SubscriptionOption | undefined;
  /** The Always rule's term while it is on. */
  ruleTerm: string | null;
}

export function lineChoices(line: ReviewLine, { itemLabel, linked, ruleTerm }: Picks, runs: Runs): Choice[] {
  const text = t().review.choices;
  const also = ruleTerm ? text.also(ruleTerm) : "";
  const base = (action: Action) => ({ key: KEYS[action], ...text[action], action, suggested: action === line.suggestion });
  return [
    {
      ...base("anv"),
      confirm: text.countAsAnv,
      ...(itemLabel ? { outcome: text.anvOutcome(itemLabel, also) } : { outcome: "", blocked: text.chooseItem }),
      run: () => runs.decide("anv", null),
    },
    linked
      ? { ...base("subscription"), confirm: text.linkTo(linked.name), outcome: text.becomesCharge(linked.name), run: () => runs.decide("subscription", linked.id) }
      : { ...base("subscription"), confirm: text.setUp, outcome: text.setUpOutcome, run: runs.setUp },
    {
      ...base("ignore"),
      confirm: text.ignoreLine,
      outcome: text.ignoreOutcome(also),
      run: () => runs.decide("ignore", null),
    },
  ];
}
