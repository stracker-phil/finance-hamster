import { useRef, useState } from "react";
import { Check } from "lucide-react";
import { api, toAppError, type Action, type ReviewLine } from "../api";
import { usePageKeys } from "../commands/pageKeys";
import { useReviewQueue } from "../queries/review";
import { lineChoices, planChoices, type Choice } from "../review/choices";
import { NEW_SUBSCRIPTION, ReviewCard } from "../review/ReviewCard";
import { useStore } from "../state/store";
import { t, useT } from "../i18n";
import { changePlan, newSubscriptionFrom } from "../subscriptions/flow";

const SKIP_KEY = "J";

const undo = () => {
  const { showToast } = useStore.getState();
  api
    .undo()
    .then((label) => label && showToast("success", t().pages.review.undid(label)))
    .catch((e: unknown) => showToast("error", toAppError(e).message));
};

function AllReviewed({ decided, rules }: { decided: number; rules: number }) {
  const navigate = useStore((s) => s.navigate);
  const text = useT().pages.review;
  const line = decided === 0 ? text.noneWaiting : text.decided(decided, rules);
  return (
    <div className="empty-page">
      <span className="review-done__tile">
        <Check size={22} strokeWidth={2.6} />
      </span>
      <div className="empty-page__title">{text.allReviewed}</div>
      <div>{line}</div>
      <div className="review-done__actions">
        <button className="btn" onClick={() => navigate({ page: "statements" })}>
          {text.backToStatements}
        </button>
        <button className="btn btn--primary" onClick={() => navigate({ page: "today" })}>
          {text.backToToday}
        </button>
      </div>
    </div>
  );
}

export function ReviewPage() {
  const { data } = useReviewQueue();
  const [position, setPosition] = useState(0);
  const [picks, setPicks] = useState<Record<string, string>>({});
  const [items, setItems] = useState<Record<string, string>>({});
  const [always, setAlways] = useState<Record<string, boolean>>({});
  const [subscriptions, setSubscriptions] = useState<Record<string, string>>({});
  const [terms, setTerms] = useState<Record<string, string>>({});
  /** Lines answered "No, not this subscription": asked like any other. */
  const [notThis, setNotThis] = useState<Record<string, true>>({});
  const [session, setSession] = useState({ decided: 0, rules: 0 });
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const itemSelect = useRef<HTMLButtonElement>(null);
  const text = useT().pages.review;
  const lines = data?.lines ?? [];
  const index = Math.min(position, Math.max(lines.length - 1, 0));
  const line = lines[index] as ReviewLine | undefined;
  const current: ReviewLine | undefined = line && notThis[line.id] ? { ...line, planChanged: null } : line;

  const itemOf = (line: ReviewLine) => items[line.id] ?? line.itemKey ?? "";

  const alwaysOn = current ? (always[current.id] ?? false) : false;
  const pickedSubscription = current ? (subscriptions[current.id] ?? NEW_SUBSCRIPTION) : NEW_SUBSCRIPTION;
  /** The owner's selected text, sent only when it differs from the line's key. */
  const chosenTerm = current ? (terms[current.id] ?? null) : null;
  const term = current ? (chosenTerm ?? current.key) : "";

  const sendDecision = (action: Action, subscriptionId: string | null) => {
    if (!current || busy) return;
    // Rust makes no rule for a Subscription decision.
    const rule = alwaysOn && action !== "subscription";
    setBusy(true);
    setError(null);
    api
      .reviewDecide(current.id, action, itemOf(current) || null, subscriptionId, rule, chosenTerm)
      .then((label) => {
        setSession((s) => ({ decided: s.decided + 1, rules: s.rules + (rule ? 1 : 0) }));
        useStore.getState().showToast("success", label, { label: t().pages.review.undo, run: undo });
      })
      .catch((e: unknown) => {
        const failure = toAppError(e);
        setError(failure.field?.message ?? failure.message);
        if (failure.field?.path === "review.item") itemSelect.current?.focus();
      })
      .finally(() => setBusy(false));
  };

  const updatePlan = (line: ReviewLine) => {
    const plan = line.planChanged;
    if (!plan) return;
    api
      .subscriptionDetail(plan.subscriptionId)
      .then((detail) => changePlan(detail, plan, line))
      .catch((e: unknown) => setError(toAppError(e).message));
  };

  const choices = (line: ReviewLine): Choice[] => {
    const runs = {
      decide: sendDecision,
      updatePlan: () => updatePlan(line),
      setUp: () => newSubscriptionFrom(line, chosenTerm),
      notThis: () => {
        setNotThis((m) => ({ ...m, [line.id]: true }));
        setPicks(({ [line.id]: _, ...rest }) => rest);
      },
    };
    if (line.planChanged) return planChoices(line.planChanged, runs);
    return lineChoices(
      line,
      {
        itemLabel: data?.items.find((i) => i.key === itemOf(line))?.label,
        linked: data?.subscriptions.find((s) => s.id === pickedSubscription),
        ruleTerm: alwaysOn ? term : null,
      },
      runs,
    );
  };
  const shown = current ? choices(current) : [];
  const picked = current ? (picks[current.id] ?? shown.find((c) => c.suggested)?.key ?? null) : null;
  const chosen = shown.find((c) => c.key === picked);

  const pick = (key: string) => {
    if (!current) return;
    setError(null);
    setPicks((m) => ({ ...m, [current.id]: key }));
  };
  const confirm = () => {
    if (chosen && !chosen.blocked && !busy) chosen.run();
  };
  const skip = () => {
    if (lines.length > 0) setPosition((index + 1) % lines.length);
  };
  const previous = () => {
    if (index > 0) setPosition(index - 1);
  };

  usePageKeys({ ...Object.fromEntries(shown.map((c) => [c.key.toLowerCase(), () => pick(c.key)])), enter: confirm, [SKIP_KEY.toLowerCase()]: skip });

  if (!data) return null;
  if (!current) return <AllReviewed decided={session.decided} rules={session.rules} />;
  const total = session.decided + lines.length;
  return (
    <div className="page-scroll review">
      <div className="review__progress" role="progressbar" aria-label={text.reviewed} aria-valuenow={session.decided} aria-valuemax={total}>
        <div className="review__bar" style={{ width: `${(session.decided / total) * 100}%` }} />
      </div>
      <div className="review__column">
        <div className="review__count">
          {text.count(lines.length)}
        </div>
        <ReviewCard
          line={current}
          choices={shown}
          picked={picked}
          items={data.items}
          subscriptions={data.subscriptions}
          item={itemOf(current)}
          itemChosen={current.id in items}
          subscription={pickedSubscription}
          always={alwaysOn}
          term={term}
          error={error}
          busy={busy}
          itemSelect={itemSelect}
          onPick={pick}
          onConfirm={confirm}
          onItem={(key) => setItems((m) => ({ ...m, [current.id]: key }))}
          onSubscription={(id) => setSubscriptions((m) => ({ ...m, [current.id]: id }))}
          onAlways={(on) => setAlways((m) => ({ ...m, [current.id]: on }))}
          onTerm={(text) => {
            setError(null);
            setTerms(({ [current.id]: _, ...rest }) => (text === null || text.toUpperCase() === current.key ? rest : { ...rest, [current.id]: text.toUpperCase() }));
          }}
        />
        <div className="review__nav">
          <button className="link-btn" onClick={previous} disabled={index === 0}>
            {text.previous}
          </button>
          <span className="review__keys">
            {shown.map((c) => (
              <span key={c.key} className="kbd">
                {c.key}
              </span>
            ))}{" "}
            {text.pick} · <span className="kbd">↩</span> {text.confirm} · <span className="kbd">{SKIP_KEY}</span> {text.skipKey}
          </span>
          <button className="link-btn" onClick={skip} disabled={lines.length < 2}>
            {text.skip}
          </button>
        </div>
      </div>
    </div>
  );
}
