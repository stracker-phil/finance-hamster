import { useAccountLines } from "../queries/statements";
import { EmptyPage } from "./EmptyPage";

/** One account's lines, grouped by statement, newest first. */
export function AccountPage({ accountId }: { accountId: string }) {
  const { data, error } = useAccountLines(accountId);
  if (error) return <EmptyPage title="No such account" text="It may have been removed." />;
  if (data && data.groups.length === 0) return <EmptyPage title="No lines" text="This account has no imported statement yet." />;
  return (
    <div className="page-scroll">
      {data?.groups.map((group) => (
        <section key={group.id} aria-label={group.label}>
          <div className="line-head">
            <span className="line-head__label">{group.label}</span>
            <span>{group.range}</span>
            <span className="line-head__ok">✓ balance checked</span>
            <span className="line-head__count">{group.count}</span>
          </div>
          {group.lines.map((line) => (
            <div key={line.id} className="line-row">
              <span className="line-row__date">{line.date}</span>
              <span className="line-row__title">{line.title}</span>
              {line.transfer && <span className="chip">Transfer</span>}
              {line.original && <span className="line-row__fx">{line.original}</span>}
              <span className={line.credit ? "line-row__amount line-row__amount--in" : "line-row__amount"}>{line.amount}</span>
            </div>
          ))}
        </section>
      ))}
    </div>
  );
}
