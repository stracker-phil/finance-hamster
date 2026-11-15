import { FileText } from "lucide-react";
import type { DocState } from "../../api";
import { useT } from "../../i18n";
import { useDocuments } from "../../queries/documents";
import { useStore } from "../../state/store";

const STATE_PILL: Record<DocState, string> = {
  draft: "pill pill--dashed",
  finished: "pill pill--blue",
  excluded: "pill",
};

/** Invoices by tax year, the undated ones in the Inbox. */
export function InvoiceList({ query, empty }: { query: string; empty: React.ReactNode }) {
  const { data: groups } = useDocuments(query);
  const navigate = useStore((s) => s.navigate);
  const text = useT().pages.documents;
  if (groups?.length === 0) return <>{empty}</>;
  return (
    <>
      {groups?.map((group) => (
        <section key={group.title} className="docs-group" aria-label={group.title}>
          <div className="docs-group__head">
            {group.year ? (
              <button className="link-btn docs-group__title" onClick={() => group.year && navigate({ page: "year", year: group.year })}>
                {group.title}
              </button>
            ) : (
              <span className="docs-group__title">{group.title}</span>
            )}
            <span className="add-doc__hint">{group.count}</span>
            {group.year && (
              <>
                <button className="link-btn docs-group__link" onClick={() => group.year && navigate({ page: "log", year: group.year })}>
                  {text.log}
                </button>
                <button className="link-btn docs-group__link" onClick={() => group.year && navigate({ page: "archive", year: group.year })}>
                  {text.archive}
                </button>
              </>
            )}
            {group.total && (
              <span className="docs-group__total" title={text.totalTip}>
                {group.total}
              </span>
            )}
          </div>
          {group.documents.map((doc) => (
            <button key={doc.id} className="docs-row" onClick={() => navigate({ page: "document", documentId: doc.id, year: null })}>
              <FileText size={17} strokeWidth={1.8} className="docs-row__icon" />
              <span className="docs-row__text">
                <span className="docs-row__title">{doc.title}</span>
                <span className="add-doc__hint">{doc.sub}</span>
              </span>
              <span className={STATE_PILL[doc.state]}>{text[doc.state]}</span>
              <span className="docs-row__anv">{doc.anv}</span>
            </button>
          ))}
        </section>
      ))}
    </>
  );
}
