import { CreditCard, X } from "lucide-react";
import { accountTone } from "../lib/accountTone";
import { useSubscription } from "../queries/subscriptions";
import { changePlan, editSubscription, endSubscription, resumeSubscription } from "./flow";
import { useT } from "../i18n";

/** How many timeline colours there are (`.plan-tone-N`); later plans reuse them. */
const PLAN_TONES = 4;

/** The detail panel beside the list: amount, next charge, the year's plans, ANV, reminder, rule and charges. */
export function SubscriptionPanel({ id, onClose }: { id: string; onClose: () => void }) {
  const { data: detail } = useSubscription(id);
  const text = useT().subscriptions.panel;
  return (
    <aside className="sub-panel" aria-label={text.subscription}>
      <div className="sub-panel__bar">
        <span className="sub-panel__caption">{text.subscription}</span>
        <button className="icon-btn" aria-label={text.close} onClick={onClose}>
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
          <div className="add-doc__hint">{detail.next ? text.next(detail.next) : detail.ended}</div>

          {detail.plans.length > 0 && (
            <section aria-label={text.plansIn(detail.year)}>
              <h3 className="sub-sheet__heading">{text.plansIn(detail.year)}</h3>
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
            <dt>{text.countsForAnv}</dt>
            <dd>{detail.anv}</dd>
            <dt>{text.reminder}</dt>
            <dd>{detail.reminder}</dd>
            <dt>{text.rule}</dt>
            <dd className="mono">{detail.rule}</dd>
          </dl>

          <section aria-label={text.charges}>
            <h3 className="sub-sheet__heading">{text.charges}</h3>
            {detail.charges.length === 0 && <p className="add-doc__hint">{text.noCharge}</p>}
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
                {text.resume}
              </button>
            ) : (
              <button className="btn btn--small" onClick={() => changePlan(detail)}>
                {text.changePlan}
              </button>
            )}
            <button className="btn btn--small" onClick={() => editSubscription(detail)}>
              {text.edit}
            </button>
            <span className="sub-panel__spacer" />
            {!detail.ended && (
              <button className="link-btn" onClick={() => void endSubscription(detail.id, detail.name)}>
                {text.end}
              </button>
            )}
          </div>
          <p className="add-doc__hint sub-panel__note">{text.note}</p>
        </div>
      )}
    </aside>
  );
}
