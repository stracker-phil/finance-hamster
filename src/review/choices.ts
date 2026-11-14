import type { Action, PlanChanged, ReviewLine, SubscriptionOption } from "../api";

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

export const CHOICES: { action: Action; key: string; label: string; hint: string }[] = [
  { action: "anv", key: "A", label: "ANV", hint: "Counts for the tax year" },
  { action: "subscription", key: "S", label: "Subscription", hint: "Recurring payment" },
  { action: "ignore", key: "I", label: "Ignore", hint: "Not relevant" },
];

/** What confirming runs; the page owns the writes. */
export interface Runs {
  decide: (action: Action, subscriptionId: string | null) => void;
  setUp: () => void;
  updatePlan: () => void;
}

const SET_UP = { confirm: "Set Up Subscription…", outcome: "Opens New Subscription with this line, to check before saving." };

export function planChoices(plan: PlanChanged, runs: Runs): Choice[] {
  return [
    {
      key: "U",
      label: "Update Plan",
      hint: "A new plan from this charge",
      suggested: true,
      confirm: "Update Plan…",
      outcome: `Opens Change Plan for ${plan.name} at ${plan.amount} ${plan.currency}, to check before saving.`,
      run: runs.updatePlan,
    },
    {
      key: "O",
      label: "One-off",
      hint: "Keep the plan",
      confirm: "Link as One-off",
      outcome: `Keeps the plan and links this charge to ${plan.name}.`,
      run: () => runs.decide("subscription", plan.subscriptionId),
    },
    { key: "N", label: "New Subscription", hint: "A different service", ...SET_UP, run: runs.setUp },
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
  const also = ruleTerm ? `, and so will every line containing ${ruleTerm}, also in future statements` : "";
  const base = (action: Action) => {
    const { key, label, hint } = CHOICES.find((c) => c.action === action) ?? CHOICES[0];
    return { key, label, hint, action, suggested: action === line.suggestion };
  };
  return [
    {
      ...base("anv"),
      confirm: "Count as ANV",
      ...(itemLabel ? { outcome: `It counts for ${itemLabel} in its tax year${also}.` } : { outcome: "", blocked: "Choose the item it counts for." }),
      run: () => runs.decide("anv", null),
    },
    linked
      ? { ...base("subscription"), confirm: `Link to ${linked.name}`, outcome: `It becomes a charge of ${linked.name}.`, run: () => runs.decide("subscription", linked.id) }
      : { ...base("subscription"), ...SET_UP, run: runs.setUp },
    {
      ...base("ignore"),
      confirm: "Ignore Line",
      outcome: `It stays in the statement and needs nothing more${also}.`,
      run: () => runs.decide("ignore", null),
    },
  ];
}
