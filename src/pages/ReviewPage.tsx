import { useRef, useState, type RefObject } from "react";
import { Check } from "lucide-react";
import { api, toAppError, type Action, type ItemOption, type ReviewLine } from "../api";
import { usePageKeys } from "../commands/pageKeys";
import { accountTone } from "../lib/accountTone";
import { useReviewQueue } from "../queries/review";
import { useStore } from "../state/store";

const CHOICES: { action: Action; key: string; label: string; hint: string }[] = [
  { action: "anv", key: "A", label: "ANV", hint: "Counts for the tax year" },
  { action: "subscription", key: "S", label: "Subscription", hint: "Recurring payment" },
  { action: "ignore", key: "I", label: "Ignore", hint: "Not relevant" },
];

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

interface CardProps {
  line: ReviewLine;
  items: ItemOption[];
  item: string;
  itemChosen: boolean;
  always: boolean;
  error: string | null;
  busy: boolean;
  itemSelect: RefObject<HTMLSelectElement | null>;
  onDecide: (action: Action) => void;
  onItem: (key: string) => void;
  onAlways: (on: boolean) => void;
}

function ReviewCard({ line, items, item, itemChosen, always, error, busy, itemSelect, onDecide, onItem, onAlways }: CardProps) {
  const suggested = CHOICES.find((c) => c.action === line.suggestion);
  return (
    <article className="review-card" aria-label={line.title}>
      <div className="review-card__account">
        <span className={`tile tile--small ${accountTone(line.accountIndex)}`}>{line.accountName.slice(0, 1).toUpperCase()}</span>
        {line.accountName} · {line.date}
      </div>
      <div className="review-card__head">
        <h2 className="review-card__title">{line.title}</h2>
        <div className="review-card__amount">{line.amount}</div>
      </div>
      <pre className="review-card__raw">{line.raw}</pre>
      <div className="review-card__choices">
        {CHOICES.map((c) => (
          <button key={c.action} className={c === suggested ? "tbig tbig--suggested" : "tbig"} aria-label={`${c.label} (${c.key})`} disabled={busy} onClick={() => onDecide(c.action)}>
            <span className="kbd">{c.key}</span>
            <span className="tbig__label">{c.label}</span>
            <span className="tbig__hint">{c.hint}</span>
          </button>
        ))}
      </div>
      <div className="review-card__item">
        <label htmlFor="review-item">If ANV, counts as</label>
        <select id="review-item" ref={itemSelect} className="field" value={item} onChange={(e) => onItem(e.target.value)}>
          <option value="">Choose an item</option>
          {items.map((i) => (
            <option key={i.key} value={i.key}>
              {i.label}
            </option>
          ))}
        </select>
        {line.itemWhy && !itemChosen && <span className="review-card__why">{line.itemWhy}</span>}
      </div>
      <div className="form-field__error review-card__error" role={error ? "alert" : undefined}>
        {error ?? ""}
      </div>
      {suggested && (
        <div className="review-card__suggestion">
          Suggested: {suggested.label}, as before for “{line.key}”
        </div>
      )}
      <label className="review-card__always">
        <input type="checkbox" checked={always} onChange={(e) => onAlways(e.target.checked)} />
        Always do this for “{line.key}”
      </label>
    </article>
  );
}

export function ReviewPage() {
  const { data } = useReviewQueue();
  const [position, setPosition] = useState(0);
  const [items, setItems] = useState<Record<string, string>>({});
  const [always, setAlways] = useState<Record<string, boolean>>({});
  const [session, setSession] = useState({ decided: 0, rules: 0 });
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const itemSelect = useRef<HTMLSelectElement>(null);
  const lines = data?.lines ?? [];
  const index = Math.min(position, Math.max(lines.length - 1, 0));
  const current: ReviewLine | undefined = lines[index];

  const itemOf = (line: ReviewLine) => items[line.id] ?? line.itemKey ?? "";

  const decide = (action: Action) => {
    if (!current || busy) return;
    const rule = always[current.id] ?? false;
    setBusy(true);
    setError(null);
    api
      .reviewDecide(current.id, action, itemOf(current) || null, rule)
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

  const skip = () => {
    if (lines.length > 0) setPosition((index + 1) % lines.length);
  };
  const previous = () => {
    if (index > 0) setPosition(index - 1);
  };

  usePageKeys({ ...Object.fromEntries(CHOICES.map((c) => [c.key.toLowerCase(), () => decide(c.action)])), [SKIP_KEY.toLowerCase()]: skip });

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
          items={data.items}
          item={itemOf(current)}
          itemChosen={current.id in items}
          always={always[current.id] ?? false}
          error={error}
          busy={busy}
          itemSelect={itemSelect}
          onDecide={decide}
          onItem={(key) => setItems((m) => ({ ...m, [current.id]: key }))}
          onAlways={(on) => setAlways((m) => ({ ...m, [current.id]: on }))}
        />
        <div className="review__nav">
          <button className="link-btn" onClick={previous} disabled={index === 0}>
            ← Previous
          </button>
          <span className="review__keys">
            {CHOICES.map((c) => (
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
