import { ChevronRight, Upload } from "lucide-react";
import type { AccountCoverage } from "../api";
import { useCoverage } from "../queries/statements";
import { startImport } from "../statements/importFlow";
import { useStore } from "../state/store";
import { accountTone } from "../lib/accountTone";
import { EmptyPage } from "./EmptyPage";
import { useT } from "../i18n";

const STATE_CHIP: Record<AccountCoverage["state"], string> = { ok: "state-chip--ok", behind: "state-chip--warn", gap: "state-chip--warn", review: "state-chip--info" };

function AccountRow({ account, index }: { account: AccountCoverage; index: number }) {
  const navigate = useStore((s) => s.navigate);
  const tone = accountTone(index);
  const text = useT().pages.statements;
  return (
    <div className="cov-row">
      <button className="cov-row__open" onClick={() => navigate({ page: "account", accountId: account.id })} aria-label={text.open(account.name)} />
      <div className="cov-row__main">
        <div className="cov-row__name">
          <span className={`tile ${tone}`}>{account.name.slice(0, 1).toUpperCase()}</span>
          <div>
            <div>{account.name}</div>
            <div className="cov-row__last">{account.last}</div>
          </div>
        </div>
        <div className={`cov-row__months ${tone}`}>
          {account.months.map((m, i) => (
            <span key={i} className={`cov cov--${m.cover}`} title={m.tip} />
          ))}
        </div>
        <div className="cov-row__state">
          <span className={`state-chip ${STATE_CHIP[account.state]}`}>{account.stateLabel}</span>
          <ChevronRight size={14} />
        </div>
      </div>
      {account.note && (
        <div className="cov-row__note">
          <span className="cov-row__dot" />
          {account.note}
        </div>
      )}
    </div>
  );
}

/** Coverage per account over the last twelve months. */
export function StatementsPage() {
  const filter = useStore((s) => s.filter);
  const { data } = useCoverage(filter);
  const navigate = useStore((s) => s.navigate);
  const text = useT().pages.statements;
  if (data && data.accounts.length === 0) return <EmptyPage title={text.emptyTitle} text={text.emptyText} />;
  const toReview = data?.accounts.reduce((sum, a) => sum + a.toReview, 0) ?? 0;
  return (
    <div className="page-scroll">
      <div className="statements">
        {toReview > 0 && (
          <div className="review-banner">
            <span>{text.waiting(toReview)}</span>
            <button className="btn btn--small btn--primary" onClick={() => navigate({ page: "review" })}>
              {text.startReview}
            </button>
          </div>
        )}
        <div className="dropline">
          <Upload size={18} />
          <span>{text.drop}</span>
          <button className="btn btn--small" onClick={startImport}>
            {text.import}
          </button>
        </div>
        <div className="cov-head">
          <span className="cov-head__lead" />
          {data?.months.map((m, i) => (
            <span key={i} className={m.january ? "cov-head__month cov-head__month--jan" : "cov-head__month"}>
              {m.label}
            </span>
          ))}
          <span className="cov-head__tail" />
        </div>
        {data?.accounts.map((account, i) => <AccountRow key={account.id} account={account} index={i} />)}
        <div className="cov-legend">
          <span>
            <span className="cov cov--full" />
            {text.imported}
          </span>
          <span>
            <span className="cov cov--part" />
            {text.partly}
          </span>
          <span>
            <span className="cov cov--missing" />
            {text.missing}
          </span>
          <span>
            <span className="cov cov--before" />
            {text.before}
          </span>
          <span className="cov-legend__note">{text.legendNote}</span>
        </div>
      </div>
    </div>
  );
}
