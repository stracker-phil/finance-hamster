import { ChevronRight, Upload } from "lucide-react";
import type { AccountCoverage } from "../api";
import { useCoverage } from "../queries/statements";
import { startImport } from "../statements/importFlow";
import { useStore } from "../state/store";
import { EmptyPage } from "./EmptyPage";

/** The number of `acct-N` color classes in statements.css. */
const ACCOUNT_TONES = 6;

function AccountRow({ account, index }: { account: AccountCoverage; index: number }) {
  const navigate = useStore((s) => s.navigate);
  const tone = `acct-${index % ACCOUNT_TONES}`;
  return (
    <div className="cov-row">
      <button className="cov-row__open" onClick={() => navigate({ page: "account", accountId: account.id })} aria-label={`Open ${account.name}`} />
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
          <span className={account.state === "ok" ? "state-chip state-chip--ok" : "state-chip state-chip--warn"}>{account.stateLabel}</span>
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
  const { data } = useCoverage();
  if (data && data.accounts.length === 0) return <EmptyPage title="No accounts" text="Add an account in Settings → Accounts." />;
  return (
    <div className="page-scroll">
      <div className="statements">
        <div className="dropline">
          <Upload size={18} />
          <span>Drop a statement PDF anywhere in this window. The app finds the account from the IBAN.</span>
          <button className="btn btn--small" onClick={startImport}>
            Import…
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
            Imported
          </span>
          <span>
            <span className="cov cov--part" />
            Partly
          </span>
          <span>
            <span className="cov cov--missing" />
            Missing
          </span>
          <span className="cov-legend__note">Checked by printed balances · statements must follow each other</span>
        </div>
      </div>
    </div>
  );
}
