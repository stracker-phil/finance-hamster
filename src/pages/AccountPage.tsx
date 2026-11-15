import { useEffect, useState } from "react";
import type { LineStatus, LineView } from "../api";
import { ChangeLineSheet } from "../statements/ChangeLineSheet";
import { useAccountLines } from "../queries/statements";
import { useStore } from "../state/store";
import { EmptyPage } from "./EmptyPage";
import { useT } from "../i18n";

/** Lines that don't count: ignored, transfers, and those still waiting for Review. */
const MUTED: LineStatus[] = ["ignored", "transfer", "review"];

/** One account's lines, grouped by statement, newest first. */
export function AccountPage({ accountId, statementId }: { accountId: string; statementId?: string }) {
  const filter = useStore((s) => s.filter);
  const { data, error } = useAccountLines(accountId, filter);
  const navigate = useStore((s) => s.navigate);
  const [changing, setChanging] = useState<LineView | null>(null);
  const text = useT().pages.account;
  const loaded = data !== undefined;
  // Opened from a statement: its lines come into view once they're there.
  useEffect(() => {
    if (loaded && statementId) document.getElementById(`statement-${statementId}`)?.scrollIntoView?.({ block: "start" });
  }, [loaded, statementId]);
  // A transfer between own accounts is decided by the app; a line still waiting goes to Review.
  const open = (line: LineView) => (line.status === "review" ? navigate({ page: "review" }) : setChanging(line));
  if (error) return <EmptyPage title={text.missingTitle} text={text.missingText} />;
  if (data && data.groups.length === 0) return <EmptyPage title={text.emptyTitle} text={text.emptyText} />;
  return (
    <div className="page-scroll">
      {data?.groups.map((group) => (
        <section key={group.id} id={`statement-${group.id}`} aria-label={group.label}>
          <div className="line-head">
            <button type="button" className="link-btn line-head__label" title={text.openStatement} onClick={() => navigate({ page: "statement", statementId: group.id })}>
              {group.label}
            </button>
            <span>{group.range}</span>
            <span className="line-head__ok">{text.balanceChecked}</span>
            <span className="line-head__count">{group.count}</span>
          </div>
          {group.lines.map((line) => (
            <button
              key={line.id}
              type="button"
              className={MUTED.includes(line.status) ? "line-row line-row--muted" : "line-row"}
              disabled={line.status === "transfer"}
              aria-label={text.change(line.title)}
              onClick={() => open(line)}
            >
              <span className="line-row__date">{line.date}</span>
              <span className="line-row__title">{line.title}</span>
              {line.status === "anv" && <span className="chip chip--anv">ANV</span>}
              {line.status === "subscription" && <span className="chip chip--sub">{text.subscription}</span>}
              {line.transfer && <span className="chip">{text.transfer}</span>}
              {line.note && <span className="line-row__note">{line.note}</span>}
              {line.original && <span className="line-row__fx">{line.original}</span>}
              <span className={line.credit ? "line-row__amount line-row__amount--in" : "line-row__amount"}>{line.amount}</span>
            </button>
          ))}
          {group.missingBefore && (
            <div className="line-gap" role="note">
              <span className="cov-row__dot" />
              {group.missingBefore}
            </div>
          )}
        </section>
      ))}
      {changing && <ChangeLineSheet line={changing} onClose={() => setChanging(null)} />}
    </div>
  );
}
