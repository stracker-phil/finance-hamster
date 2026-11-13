import { useRef, useState } from "react";
import { Check } from "lucide-react";
import { api, toAppError, type Action, type ReviewLine } from "../api";
import { usePageKeys } from "../commands/pageKeys";
import { useReviewQueue } from "../queries/review";
import { CHOICES, NEW_SUBSCRIPTION, ReviewCard, type Choice } from "../review/ReviewCard";
import { useStore } from "../state/store";
import { changePlan, newSubscriptionFrom } from "../subscriptions/flow";

const SKIP_KEY = "J";

const undo = () => {
  const { showToast } = useStore.getState();
  api
    .undo()
    .then((label) => label && showToast("success", `Undid ${label}`))
    .catch((e: unknown) => showToast("error", toAppError(e).message));
};

function AllReviewed({ decided, rules }: { decided: number; rules: number }) {
  const navigate = useStore((s) => s.navigate);
  const line = decided === 0 ? "No line is waiting for a decision." : `${decided} decided · ${rules} new ${rules === 1 ? "rule" : "rules"}`;
  return (
    <div className="empty-page">
      <span className="review-done__tile">
        <Check size={22} strokeWidth={2.6} />
      </span>
      <div className="empty-page__title">All reviewed</div>
      <div>{line}</div>
      <div className="review-done__actions">
        <button className="btn" onClick={() => navigate({ page: "statements" })}>
          Back to Statements
        </button>
        <button className="btn btn--primary" onClick={() => navigate({ page: "today" })}>
          Back to Today
        </button>
      </div>
    </div>
  );
}

export function ReviewPage() {
  const { data } = useReviewQueue();
  const [position, setPosition] = useState(0);
  const [items, setItems] = useState<Record<string, string>>({});
  const [always, setAlways] = useState<Record<string, boolean>>({});
  const [subscriptions, setSubscriptions] = useState<Record<string, string>>({});
  const [session, setSession] = useState({ decided: 0, rules: 0 });
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const itemSelect = useRef<HTMLSelectElement>(null);
  const lines = data?.lines ?? [];
  const index = Math.min(position, Math.max(lines.length - 1, 0));
  const current: ReviewLine | undefined = lines[index];

  const itemOf = (line: ReviewLine) => items[line.id] ?? line.itemKey ?? "";

  const alwaysOn = current ? (always[current.id] ?? false) : false;
  const pickedSubscription = current ? (subscriptions[current.id] ?? NEW_SUBSCRIPTION) : NEW_SUBSCRIPTION;

  const sendDecision = (action: Action, subscriptionId: string | null) => {
    if (!current || busy) return;
    // Rust makes no rule for a Subscription decision; the done count follows it.
    const rule = alwaysOn && action !== "subscription";
    setBusy(true);
    setError(null);
    api
      .reviewDecide(current.id, action, itemOf(current) || null, subscriptionId, alwaysOn)
      .then((label) => {
        setSession((s) => ({ decided: s.decided + 1, rules: s.rules + (rule ? 1 : 0) }));
        useStore.getState().showToast("success", label, { label: "Undo", run: undo });
      })
      .catch((e: unknown) => {
        const failure = toAppError(e);
        setError(failure.field?.message ?? failure.message);
        if (failure.field?.path === "review.item") itemSelect.current?.focus();
      })
      .finally(() => setBusy(false));
  };

  const decide = (action: Action) => {
    if (!current) return;
    if (action === "subscription" && pickedSubscription === NEW_SUBSCRIPTION) newSubscriptionFrom(current.id);
    else sendDecision(action, action === "subscription" ? pickedSubscription : null);
  };

  const updatePlan = () => {
    const plan = current?.planChanged;
    if (!plan) return;
    api
      .subscriptionDetail(plan.subscriptionId)
      .then((detail) => changePlan(detail, plan))
      .catch((e: unknown) => setError(toAppError(e).message));
  };

  const choices = (line: ReviewLine): Choice[] => {
    const plan = line.planChanged;
    if (plan)
      return [
        { key: "U", label: "Update Plan", hint: "A new plan from this charge", suggested: true, run: updatePlan },
        { key: "O", label: "One-off", hint: `Keep the plan; link it to ${plan.name}`, run: () => sendDecision("subscription", plan.subscriptionId) },
        { key: "N", label: "New Subscription", hint: "A different service", run: () => newSubscriptionFrom(line.id) },
      ];
    return CHOICES.map((c) => ({ key: c.key, label: c.label, hint: c.hint, suggested: c.action === line.suggestion, run: () => decide(c.action) }));
  };
  const shown = current ? choices(current) : [];

  const skip = () => {
    if (lines.length > 0) setPosition((index + 1) % lines.length);
  };
  const previous = () => {
    if (index > 0) setPosition(index - 1);
  };

  usePageKeys({ ...Object.fromEntries(shown.map((c) => [c.key.toLowerCase(), c.run])), [SKIP_KEY.toLowerCase()]: skip });

  if (!data) return null;
  if (!current) return <AllReviewed decided={session.decided} rules={session.rules} />;
  const total = session.decided + lines.length;
  return (
    <div className="page-scroll review">
      <div className="review__progress" role="progressbar" aria-label="Reviewed" aria-valuenow={session.decided} aria-valuemax={total}>
        <div className="review__bar" style={{ width: `${(session.decided / total) * 100}%` }} />
      </div>
      <div className="review__column">
        <div className="review__count">
          {lines.length} {lines.length === 1 ? "line needs" : "lines need"} your decision
        </div>
        <ReviewCard
          line={current}
          choices={shown}
          items={data.items}
          subscriptions={data.subscriptions}
          item={itemOf(current)}
          itemChosen={current.id in items}
          subscription={pickedSubscription}
          always={alwaysOn}
          error={error}
          busy={busy}
          itemSelect={itemSelect}
          onItem={(key) => setItems((m) => ({ ...m, [current.id]: key }))}
          onSubscription={(id) => setSubscriptions((m) => ({ ...m, [current.id]: id }))}
          onAlways={(on) => setAlways((m) => ({ ...m, [current.id]: on }))}
        />
        <div className="review__nav">
          <button className="link-btn" onClick={previous} disabled={index === 0}>
            ← Previous
          </button>
          <span className="review__keys">
            {shown.map((c) => (
              <span key={c.key} className="kbd">
                {c.key}
              </span>
            ))}{" "}
            decide · <span className="kbd">{SKIP_KEY}</span> skip
          </span>
          <button className="link-btn" onClick={skip} disabled={lines.length < 2}>
            Skip →
          </button>
        </div>
      </div>
    </div>
  );
}
