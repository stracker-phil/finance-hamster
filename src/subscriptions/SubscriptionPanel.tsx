import { CreditCard, X } from "lucide-react";
import { accountTone } from "../lib/accountTone";
import { useSubscription } from "../queries/subscriptions";
import { changePlan, editSubscription, endSubscription, resumeSubscription } from "./flow";

/** How many timeline colours there are (`.plan-tone-N`); later plans reuse them. */
const PLAN_TONES = 4;

/** The detail panel beside the list: amount, next charge, the year's plans, ANV, reminder, rule and charges. */
export function SubscriptionPanel({ id, onClose }: { id: string; onClose: () => void }) {
  const { data: detail } = useSubscription(id);
  return (
    <aside className="sub-panel" aria-label="Subscription">
      <div className="sub-panel__bar">
        <span className="sub-panel__caption">Subscription</span>
        <button className="icon-btn" aria-label="Close (Esc)" onClick={onClose}>
          <X size={15} />
        </button>
      </div>
      {detail && (
        <div className="sub-panel__body">
          <div className="sub-panel__head">
            <span className={`tile sub-panel__tile ${accountTone(detail.tone)}`}>{detail.letter}</span>
            <div>
              <div className="sub-panel__name">{detail.name}</div>
              <div className="add-doc__hint">{detail.interval}</div>
            </div>
          </div>
          <div className="sub-panel__amount">{detail.amount}</div>
          <div className="add-doc__hint">{detail.next ? `Next charge ${detail.next}` : detail.ended}</div>

          {detail.plans.length > 0 && (
            <section aria-label={`Plans in ${detail.year}`}>
              <h3 className="sub-sheet__heading">Plans in {detail.year}</h3>
              <div className="plan-timeline">
                {detail.timeline.map((month, i) => (
                  <span key={i} className={month.plan === null ? "plan-seg" : `plan-seg plan-tone-${month.plan % PLAN_TONES}`} title={month.tip} />
                ))}
              </div>
              <div className="plan-timeline plan-timeline--labels" aria-hidden>
                {detail.timeline.map((month, i) => (
                  <span key={i}>{month.label}</span>
                ))}
              </div>
              {detail.plans.map((plan, i) => (
                <div key={i} className="sub-line">
                  <span className={`year-dot plan-tone-${i % PLAN_TONES}`} />
                  <span className="sub-line__main">
                    <b>{plan.name}</b>
                    <span className="add-doc__hint">{plan.price}</span>
                  </span>
                  <span className="add-doc__hint">{plan.when}</span>
                </div>
              ))}
            </section>
          )}

          <dl className="sub-facts">
            <dt>Counts for ANV</dt>
            <dd>{detail.anv}</dd>
            <dt>Reminder</dt>
            <dd>{detail.reminder}</dd>
            <dt>Found by rule</dt>
            <dd className="mono">{detail.rule}</dd>
          </dl>

          <section aria-label="Charges found in statements">
            <h3 className="sub-sheet__heading">Charges found in statements</h3>
            {detail.charges.length === 0 && <p className="add-doc__hint">No charge found yet.</p>}
            {detail.charges.map((charge) => (
              <div key={charge.id} className="sub-line">
                <CreditCard size={14} className="docs-row__icon" />
                <span className="sub-line__main">{charge.date}</span>
                <span className="add-doc__hint">{charge.account}</span>
                <span className="year-row__amount">{charge.amount}</span>
              </div>
            ))}
          </section>

          <div className="sub-panel__actions">
            {detail.ended ? (
              <button className="btn btn--small" onClick={() => resumeSubscription(detail.id)}>
                Resume
              </button>
            ) : (
              <button className="btn btn--small" onClick={() => changePlan(detail)}>
                Change Plan…
              </button>
            )}
            <button className="btn btn--small" onClick={() => editSubscription(detail)}>
              Edit…
            </button>
            <span className="sub-panel__spacer" />
            {!detail.ended && (
              <button className="link-btn" onClick={() => void endSubscription(detail.id, detail.name)}>
                End Subscription…
              </button>
            )}
          </div>
          <p className="add-doc__hint">Change Plan keeps the history: the old plan ends, the new one starts. Edit only fixes mistakes.</p>
        </div>
      )}
    </aside>
  );
}
