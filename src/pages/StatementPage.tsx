import { DetailLayout } from "../documents/DetailLayout";
import { useT } from "../i18n";
import { useStatement } from "../queries/statements";
import { useStore } from "../state/store";
import { EmptyPage } from "./EmptyPage";

/** A bank statement: its PDF over most of the page, what it says in the sidebar. Nothing about it changes. */
export function StatementPage({ statementId }: { statementId: string }) {
  const { data: statement, isError } = useStatement(statementId);
  const navigate = useStore((s) => s.navigate);
  const text = useT().payslips;
  const titles = useT().shell.titles;
  if (isError) return <EmptyPage title={text.statementMissingTitle} text={text.missingText} />;
  return (
    <DetailLayout source={{ kind: "statement", id: statementId }} label={statement?.title ?? titles.statement}>
      {statement && (
        <>
          <h2 className="detail__title">{statement.title}</h2>
          <dl className="detail__facts">
            {statement.facts.map((f) => (
              <div key={f.label} className="detail__fact">
                <dt>{f.label}</dt>
                <dd>{f.value}</dd>
              </div>
            ))}
          </dl>
          <button type="button" className="btn btn--small" onClick={() => navigate({ page: "account", accountId: statement.accountId, statementId })}>
            {text.statementLines}
          </button>
          <p className="add-doc__hint">{text.pdfHint}</p>
        </>
      )}
    </DetailLayout>
  );
}
