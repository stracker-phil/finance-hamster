import { Landmark } from "lucide-react";
import { useStatementsList } from "../../queries/statements";
import { useStore } from "../../state/store";

/** Every imported statement by account, newest first. */
export function StatementList({ query, empty }: { query: string; empty: React.ReactNode }) {
  const { data: accounts } = useStatementsList(query);
  const navigate = useStore((s) => s.navigate);
  if (accounts?.length === 0) return <>{empty}</>;
  return (
    <>
      {accounts?.map((account) => (
        <section key={account.accountId} className="docs-group" aria-label={account.name}>
          <div className="docs-group__head">
            <button className="link-btn docs-group__title" onClick={() => navigate({ page: "account", accountId: account.accountId })}>
              {account.name}
            </button>
          </div>
          {account.statements.map((s) => (
            <button key={s.id} className="docs-row" onClick={() => navigate({ page: "statement", statementId: s.id })}>
              <Landmark size={17} strokeWidth={1.8} className="docs-row__icon" />
              <span className="docs-row__text">
                <span className="docs-row__title">{s.title}</span>
                <span className="add-doc__hint">
                  {s.period} · {s.lines}
                </span>
              </span>
              <span className="docs-row__anv">{s.closing}</span>
            </button>
          ))}
        </section>
      ))}
    </>
  );
}
