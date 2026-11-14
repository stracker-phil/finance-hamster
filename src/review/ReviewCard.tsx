import type { RefObject } from "react";
import { useQuery } from "@tanstack/react-query";
import { Info } from "lucide-react";
import { api, type Action, type ItemOption, type ReviewLine, type SubscriptionOption } from "../api";
import { accountTone } from "../lib/accountTone";

export interface Choice {
  key: string;
  label: string;
  hint: string;
  suggested?: boolean;
  run: () => void;
}

export const CHOICES: { action: Action; key: string; label: string; hint: string }[] = [
  { action: "anv", key: "A", label: "ANV", hint: "Counts for the tax year" },
  { action: "subscription", key: "S", label: "Subscription", hint: "Recurring payment" },
  { action: "ignore", key: "I", label: "Ignore", hint: "Not relevant" },
];

/** The value of the subscription picker that creates a new one from the line. */
export const NEW_SUBSCRIPTION = "";

interface Props {
  line: ReviewLine;
  choices: Choice[];
  items: ItemOption[];
  subscriptions: SubscriptionOption[];
  item: string;
  itemChosen: boolean;
  subscription: string;
  always: boolean;
  /** The text an Always rule or a new subscription matches; the line's key until the owner selects other text. */
  term: string;
  error: string | null;
  busy: boolean;
  itemSelect: RefObject<HTMLSelectElement | null>;
  onItem: (key: string) => void;
  onSubscription: (id: string) => void;
  onAlways: (on: boolean) => void;
  /** Selected text of the line, or null to go back to its key. */
  onTerm: (text: string | null) => void;
}

/** The text selected inside `root`, on one line; empty when the selection is elsewhere. */
function selectedIn(root: HTMLElement): string {
  const selection = window.getSelection();
  if (!selection || selection.isCollapsed || !selection.anchorNode || !root.contains(selection.anchorNode)) return "";
  return selection.toString().split(/\s+/).join(" ").trim();
}

/** How many lines the term matches now, so the owner sees what Always would decide. */
function TermMatches({ term }: { term: string }) {
  const form = { contains: [term], amount: null, tolerance: null, accountId: null, ownCounterparty: false, action: "ignore" as const, itemKey: null };
  const { data } = useQuery({ queryKey: ["review-term", term], queryFn: () => api.rulePreview(form), placeholderData: (previous) => previous });
  return <span>{data?.text ?? ""}</span>;
}

/** One undecided line: its text, the choices and what each needs; a "Plan changed?" line offers the plan's
 *  choices instead. */
export function ReviewCard({ line, choices, items, subscriptions, item, itemChosen, subscription, always, term, error, busy, itemSelect, onItem, onSubscription, onAlways, onTerm }: Props) {
  const plan = line.planChanged;
  const suggested = CHOICES.find((c) => c.action === line.suggestion);
  const custom = term !== line.key;
  const pick = (e: React.MouseEvent<HTMLElement>) => {
    const text = selectedIn(e.currentTarget);
    if (text) onTerm(text);
  };
  return (
    <article className="review-card" aria-label={line.title}>
      <div className="review-card__account">
        <span className={`tile tile--small ${accountTone(line.accountIndex)}`}>{line.accountName.slice(0, 1).toUpperCase()}</span>
        {line.accountName} · {line.date}
      </div>
      <div className="review-card__text" onMouseUp={pick}>
        <div className="review-card__head">
          <h2 className="review-card__title">{line.title}</h2>
          <div className="review-card__amount">{line.amount}</div>
        </div>
        <pre className="review-card__raw">{line.raw}</pre>
      </div>
      {plan && (
        <div className="review-card__plan">
          <Info size={15} />
          <div>
            <b>Plan changed?</b> {plan.text}
          </div>
        </div>
      )}
      <div className="review-card__choices">
        {choices.map((c) => (
          <button key={c.key} className={c.suggested ? "tbig tbig--suggested" : "tbig"} aria-label={`${c.label} (${c.key})`} disabled={busy} onClick={c.run}>
            <span className="kbd">{c.key}</span>
            <span className="tbig__label">{c.label}</span>
            <span className="tbig__hint">{c.hint}</span>
          </button>
        ))}
      </div>
      {!plan && (
        <>
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
          <div className="review-card__item">
            <label htmlFor="review-subscription">If Subscription, it is</label>
            <select id="review-subscription" className="field" value={subscription} onChange={(e) => onSubscription(e.target.value)}>
              <option value={NEW_SUBSCRIPTION}>A new subscription…</option>
              {subscriptions.map((s) => (
                <option key={s.id} value={s.id}>
                  {s.name}
                </option>
              ))}
            </select>
          </div>
        </>
      )}
      <div className="form-field__error review-card__error" role={error ? "alert" : undefined}>
        {error ?? ""}
      </div>
      {suggested && !plan && (
        <div className="review-card__suggestion">
          Suggested: {suggested.label}, as before for “{line.key}”
        </div>
      )}
      {!plan && (
        <div className="review-card__rule">
          <label className="review-card__always">
            <input type="checkbox" checked={always} onChange={(e) => onAlways(e.target.checked)} />
            Always do this for lines containing <mark className="review-card__term">{term}</mark>
          </label>
          <div className="review-card__term-hint">
            <TermMatches term={term} />
            {custom ? (
              <button type="button" className="link-btn" onClick={() => onTerm(null)}>
                Use “{line.key}”
              </button>
            ) : (
              <span>Select text above, such as a contract number, to match on it instead.</span>
            )}
          </div>
        </div>
      )}
    </article>
  );
}
