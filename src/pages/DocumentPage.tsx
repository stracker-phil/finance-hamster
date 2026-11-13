import { useState } from "react";
import { Lock } from "lucide-react";
import { PdfPreview } from "../components/PdfPreview";
import { DocumentForm } from "../documents/DocumentForm";
import { ExcludeSheet } from "../documents/ExcludeSheet";
import { HistoryList } from "../documents/HistoryList";
import { runStep, unlock } from "../documents/lifecycle";
import { useDocument, useDocumentHistory } from "../queries/documents";
import { EmptyPage } from "./EmptyPage";

/** A document: its PDF beside the payment-first form, its History on the right. */
export function DocumentPage({ documentId }: { documentId: string }) {
  const { data: doc, isError } = useDocument(documentId);
  const { data: history = [] } = useDocumentHistory(documentId);
  const [excluding, setExcluding] = useState(false);
  if (isError) return <EmptyPage title="No such document" text="It may have been deleted, or the link is old." />;
  return (
    <div className="page-scroll">
      {doc?.finishedOn && (
        <div className="doc-banner" role="status">
          <Lock size={15} />
          <span>Finished on {doc.finishedOn}. Details are locked, so the Tax Year keeps exactly these values.</span>
          <button className="link-btn" onClick={() => void unlock(doc.id)}>
            Unlock…
          </button>
        </div>
      )}
      {doc?.excludedOn && (
        <div className="doc-banner doc-banner--excluded" role="status">
          <span>
            Excluded on {doc.excludedOn}
            {doc.excludeReason ? `: ${doc.excludeReason}` : ""}. It counts nowhere and shows only in its year&apos;s Archive.
          </span>
          <button className="link-btn" onClick={() => void runStep(doc.id, { kind: "restore" })}>
            Restore
          </button>
        </div>
      )}
      <div className="doc-page">
        <div className="doc-page__file">
          <div className="doc-page__preview">
            <PdfPreview source={{ kind: "document", id: documentId }} />
          </div>
          {doc && <div className="doc-page__path mono">{doc.file}</div>}
          <div className="add-doc__hint">Text in the PDF can be selected and copied</div>
        </div>
        {doc && <DocumentForm doc={doc} onExclude={() => setExcluding(true)} />}
        <aside className="doc-history" aria-label="History">
          <h3 className="doc-history__title">History</h3>
          <HistoryList entries={history} />
          <div className="doc-history__note">History lines are written once and never edited.</div>
        </aside>
      </div>
      {excluding && doc && <ExcludeSheet documentId={doc.id} title={doc.title} onClose={() => setExcluding(false)} />}
    </div>
  );
}
