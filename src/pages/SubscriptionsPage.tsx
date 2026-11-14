import { useEffect, useState } from "react";
import { ChevronDown, ChevronRight } from "lucide-react";
import { accountTone } from "../lib/accountTone";
import { useSubscriptions } from "../queries/subscriptions";
import { useStore } from "../state/store";
import { newSubscription, resumeSubscription } from "../subscriptions/flow";
import { SubscriptionPanel } from "../subscriptions/SubscriptionPanel";
import { EmptyPage } from "./EmptyPage";

/** Every subscription by next charge, the ended ones folded away at the bottom, and the selected one's panel. */
export function SubscriptionsPage({ open = null }: { open?: string | null }) {
  const filter = useStore((s) => s.filter);
  const { data } = useSubscriptions(filter);
  const [selected, setSelected] = useState<string | null>(open);
  const [endedOpen, setEndedOpen] = useState(false);
  useEffect(() => {
    if (!selected) return;
    const close = (e: KeyboardEvent) => {
      if (e.key === "Escape" && !document.querySelector(".overlay")) setSelected(null);
    };
    window.addEventListener("keydown", close);
    return () => window.removeEventListener("keydown", close);
  }, [selected]);
  if (!data) return null;
  if (data.empty)
    return (
      <EmptyPage title="No subscriptions" text="Add one with New Subscription (⌘N), or choose Subscription for a line in Review.">
        <button className="btn btn--primary" onClick={newSubscription}>
          New Subscription
        </button>
      </EmptyPage>
    );
  const nothingShown = data.groups.length === 0 && data.ended.length === 0;
  return (
    <div className="subs">
      <div className="page-scroll">
        <div className="subs__summary add-doc__hint">{data.summary}</div>
        {nothingShown && <p className="subs__none">No subscription matches “{filter}”.</p>}
        {data.groups.map((group) => (
          <section key={group.label} aria-label={group.label}>
            <div className="line-head">
              <span className="line-head__label">{group.label}</span>
              <span className="line-head__count">{group.total}</span>
            </div>
            {group.rows.map((row) => (
              <button key={row.id} className={selected === row.id ? "sub-row sub-row--selected" : "sub-row"} aria-pressed={selected === row.id} onClick={() => setSelected(row.id)}>
                <span className={`tile ${accountTone(row.tone)}`}>{row.letter}</span>
                <span className="sub-row__name">
                  {row.name}
                  {row.anv && <span className="pill pill--anv sub-row__anv">ANV</span>}
                </span>
                <span className="add-doc__hint">{row.interval}</span>
                {row.change && <span className="chip chip--miss">{row.change}</span>}
                <span className="sub-row__next">{row.next}</span>
                <span className="year-row__amount sub-row__amount">{row.amount}</span>
              </button>
            ))}
          </section>
        ))}
        {data.ended.length > 0 && (
          <section aria-label="Ended">
            <div className="line-head">
              <button className="link-btn sub-ended__toggle" aria-expanded={endedOpen} onClick={() => setEndedOpen((o) => !o)}>
                {endedOpen ? <ChevronDown size={13} /> : <ChevronRight size={13} />}
                Ended · {data.ended.length}
              </button>
              <span className="line-head__count">kept for history, never charged again</span>
            </div>
            {endedOpen &&
              data.ended.map((row) => (
                <div key={row.id} className={selected === row.id ? "sub-row sub-row--ended sub-row--selected" : "sub-row sub-row--ended"}>
                  <button className="sub-row__open" aria-label={row.name} onClick={() => setSelected(row.id)} />
                  <span className={`tile ${accountTone(row.tone)}`}>{row.letter}</span>
                  <span className="sub-row__name">{row.name}</span>
                  <span className="add-doc__hint">{row.interval}</span>
                  <span className="add-doc__hint sub-row__next">{row.ended}</span>
                  <button className="btn btn--small sub-row__resume" onClick={() => resumeSubscription(row.id)}>
                    Resume
                  </button>
                </div>
              ))}
          </section>
        )}
      </div>
      {selected && <SubscriptionPanel id={selected} onClose={() => setSelected(null)} />}
    </div>
  );
}
