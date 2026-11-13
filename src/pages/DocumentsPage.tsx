import { useState } from "react";
import { FileText, Search } from "lucide-react";
import type { DocState } from "../api";
import { useDocuments } from "../queries/documents";
import { useStore } from "../state/store";
import { EmptyPage } from "./EmptyPage";

const STATE_PILL: Record<DocState, { label: string; className: string }> = {
  draft: { label: "Draft", className: "pill pill--dashed" },
  finished: { label: "Finished", className: "pill pill--blue" },
  excluded: { label: "Excluded", className: "pill" },
};

/** Documents: "Is it already here?", then every document by tax year, the undated ones in the Inbox. */
export function DocumentsPage() {
  const [query, setQuery] = useState("");
  const filter = useStore((s) => s.filter);
  const { data: groups } = useDocuments(query || filter);
  const navigate = useStore((s) => s.navigate);
  return (
    <div className="page-scroll">
      <div className="docs-page">
        <label className="big-search">
          <Search size={17} />
          <input placeholder="Is it already here? Search vendor, invoice number or amount" aria-label="Search documents" value={query} onChange={(e) => setQuery(e.target.value)} />
        </label>
        {groups?.length === 0 &&
          (query.trim() ? (
            <p className="docs-page__none">Nothing matches “{query.trim()}”, so it isn&apos;t here yet.</p>
          ) : (
            <EmptyPage title="No documents" text="Drop an invoice PDF onto the window to add it." />
          ))}
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
                    Log
                  </button>
                  <button className="link-btn docs-group__link" onClick={() => group.year && navigate({ page: "archive", year: group.year })}>
                    Archive
                  </button>
                </>
              )}
              <span className="docs-group__total">{group.total}</span>
            </div>
            {group.documents.map((doc) => (
              <button key={doc.id} className="docs-row" onClick={() => navigate({ page: "document", documentId: doc.id, year: null })}>
                <FileText size={17} strokeWidth={1.8} className="docs-row__icon" />
                <span className="docs-row__text">
                  <span className="docs-row__title">{doc.title}</span>
                  <span className="add-doc__hint">{doc.sub}</span>
                </span>
                <span className={STATE_PILL[doc.state].className}>{STATE_PILL[doc.state].label}</span>
                <span className="docs-row__anv">{doc.anv}</span>
              </button>
            ))}
          </section>
        ))}
      </div>
    </div>
  );
}
