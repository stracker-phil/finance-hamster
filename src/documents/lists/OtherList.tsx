import { FileText } from "lucide-react";
import { useOthers } from "../../queries/others";
import { useStore } from "../../state/store";

/** Other documents by the year of their date, the undated last. */
export function OtherList({ query, empty }: { query: string; empty: React.ReactNode }) {
  const { data: groups } = useOthers(query);
  const navigate = useStore((s) => s.navigate);
  if (groups?.length === 0) return <>{empty}</>;
  return (
    <>
      {groups?.map((group) => (
        <section key={group.title} className="docs-group" aria-label={group.title}>
          <div className="docs-group__head">
            <span className="docs-group__title">{group.title}</span>
          </div>
          {group.documents.map((doc) => (
            <button key={doc.id} className="docs-row" onClick={() => navigate({ page: "other", otherId: doc.id })}>
              <FileText size={17} strokeWidth={1.8} className="docs-row__icon" />
              <span className="docs-row__text">
                <span className="docs-row__title">{doc.title}</span>
                {doc.date && <span className="add-doc__hint">{doc.date}</span>}
              </span>
            </button>
          ))}
        </section>
      ))}
    </>
  );
}
