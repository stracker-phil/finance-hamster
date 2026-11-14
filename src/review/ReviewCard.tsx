import type { RefObject } from "react";
import { useQuery } from "@tanstack/react-query";
import { Info } from "lucide-react";
import { api, type ItemOption, type ReviewLine, type SubscriptionOption } from "../api";
import { accountTone } from "../lib/accountTone";
import { selectedIn } from "../lib/selection";
import type { Choice } from "./choices";

/** The value of the subscription picker that creates a new one from the line. */
export const NEW_SUBSCRIPTION = "";

interface Props {
  line: ReviewLine;
  choices: Choice[];
  /** The key of the picked choice; null until one is picked. */
  picked: string | null;
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
  onPick: (key: string) => void;
  onConfirm: () => void;
  onItem: (key: string) => void;
  onSubscription: (id: string) => void;
  onAlways: (on: boolean) => void;
  /** Selected text of the line, or null to go back to its key. */
  onTerm: (text: string | null) => void;
}

/** How many lines the term matches now, so the owner sees what Always would decide. */
function TermMatches({ term }: { term: string }) {
  const form = { contains: [term], amount: null, tolerance: null, accountId: null, ownCounterparty: false, action: "ignore" as const, itemKey: null };
  const { data } = useQuery({ queryKey: ["review-term", term], queryFn: () => api.rulePreview(form), placeholderData: (previous) => previous });
  return <span>{data?.text ?? ""}</span>;
}

/** The picked choice's one detail row, which keeps one height whatever is picked. */
function LineDetails({ line, choice, items, item, itemChosen, subscriptions, subscription, itemSelect, onItem, onSubscription }: Pick<Props, "line" | "items" | "item" | "itemChosen" | "subscriptions" | "subscription" | "itemSelect" | "onItem" | "onSubscription"> & { choice: Choice | undefined }) {
  switch (choice?.action) {
    case "anv":
      return (
        <div className="review-card__item">
          <label htmlFor="review-item">Counts as</label>
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
      );
    case "subscription":
      return (
        <div className="review-card__item">
          <label htmlFor="review-subscription">Subscription</label>
          <select id="review-subscription" className="field" value={subscription} onChange={(e) => onSubscription(e.target.value)}>
            <option value={NEW_SUBSCRIPTION}>A new subscription…</option>
            {subscriptions.map((s) => (
              <option key={s.id} value={s.id}>
                {s.name}
              </option>
            ))}
          </select>
        </div>
      );
    default:
      return <div className="review-card__item review-card__placeholder">{choice ? "Nothing more to choose." : "Pick what this line is."}</div>;
  }
}

/** The way back to the line's own key once text was selected, else how to select some. */
function TermHint({ lineKey, term, onTerm, children }: { lineKey: string; term: string; onTerm: (text: string | null) => void; children: React.ReactNode }) {
  return (
    <div className="review-card__term-hint">
      {children}
      {term !== lineKey ? (
        <button type="button" className="link-btn" onClick={() => onTerm(null)}>
          Use “{lineKey}”
        </button>
      ) : (
        <span>Select text above, such as a contract number, to match on it instead.</span>
      )}
    </div>
  );
}

/** One undecided line: pick what it is, then confirm. Nothing is decided before the confirm button or Return. */
export function ReviewCard(props: Props) {
  const { line, choices, picked, subscription, always, term, error, busy, onPick, onConfirm, onAlways, onTerm } = props;
  const plan = line.planChanged;
  const choice = choices.find((c) => c.key === picked);
  const pick = (e: React.MouseEvent<HTMLElement>) => {
    const text = selectedIn(e.currentTarget);
    if (text) onTerm(text);
  };
  const createsRule = choice?.action === "anv" || choice?.action === "ignore";
  const newSubscription = choice?.action === "subscription" && subscription === NEW_SUBSCRIPTION;

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
      <div className="review-card__question" id="review-question">
        What is this line?
      </div>
      <div className="review-card__choices" role="radiogroup" aria-labelledby="review-question">
        {choices.map((c) => {
          const on = c.key === picked;
          return (
            <button key={c.key} type="button" role="radio" aria-checked={on} className={on ? "tbig tbig--on" : "tbig"} aria-label={`${c.label} (${c.key})`} onClick={() => onPick(c.key)}>
              <span className="kbd">{c.key}</span>
              <span className="tbig__label">{c.label}</span>
              <span className="tbig__hint">{c.suggested ? "Suggested, as before" : c.hint}</span>
            </button>
          );
        })}
      </div>
      {!plan && (
        <div className="review-card__details">
          <LineDetails {...props} choice={choice} />
          <div className="review-card__rule">
            {createsRule && (
              <>
                <label className="review-card__always">
                  <input type="checkbox" checked={always} onChange={(e) => onAlways(e.target.checked)} />
                  Also decide every line containing <mark className="review-card__term">{term}</mark> this way
                </label>
                <TermHint lineKey={line.key} term={term} onTerm={onTerm}>
                  <TermMatches term={term} />
                </TermHint>
              </>
            )}
            {newSubscription && (
              <TermHint lineKey={line.key} term={term} onTerm={onTerm}>
                <span>
                  It will find its charges by <mark className="review-card__term">{term}</mark>
                </span>
              </TermHint>
            )}
          </div>
        </div>
      )}
      <div className="review-card__confirm">
        <div className="review-card__outcome" aria-live="polite">
          {error ? (
            <span className="form-field__error" role="alert">
              {error}
            </span>
          ) : (
            (choice?.blocked ?? choice?.outcome ?? "Nothing is decided until you confirm.")
          )}
        </div>
        <button type="button" className="btn btn--primary review-card__go" disabled={!choice || !!choice.blocked || busy} onClick={onConfirm}>
          {choice?.confirm ?? "Decide"}
          <span className="kbd kbd--on-accent">↩</span>
        </button>
      </div>
    </article>
  );
}
