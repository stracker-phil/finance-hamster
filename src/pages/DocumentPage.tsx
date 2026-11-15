import { useState } from "react";
import { Lock } from "lucide-react";
import { PdfPreview } from "../components/PdfPreview";
import { DocumentForm } from "../documents/DocumentForm";
import { ExcludeSheet } from "../documents/ExcludeSheet";
import { HistoryList } from "../documents/HistoryList";
import { runStep, unlock } from "../documents/lifecycle";
import { useDocument, useDocumentHistory } from "../queries/documents";
import { EmptyPage } from "./EmptyPage";
import { useT } from "../i18n";

/** A document: its PDF beside the payment-first form, its History on the right. */
export function DocumentPage({ documentId }: { documentId: string }) {
  const { data: doc, isError } = useDocument(documentId);
  const { data: history = [] } = useDocumentHistory(documentId);
  const [excluding, setExcluding] = useState(false);
  const text = useT().pages.document;
  if (isError) return <EmptyPage title={text.missingTitle} text={text.missingText} />;
  return (
    <div className="page-scroll">
      {doc?.finishedOn && (
        <div className="doc-banner" role="status">
          <Lock size={15} />
          <span>{text.finished(doc.finishedOn)}</span>
          <button className="link-btn" onClick={() => void unlock(doc.id)}>
            {text.unlock}
          </button>
        </div>
      )}
      {doc?.excludedOn && (
        <div className="doc-banner doc-banner--excluded" role="status">
          <span>{text.excluded(doc.excludedOn, doc.excludeReason)}</span>
          <button className="link-btn" onClick={() => void runStep(doc.id, { kind: "restore" })}>
            {text.restore}
          </button>
        </div>
      )}
      <div className="doc-page">
        <div className="doc-page__file">
          <div className="doc-page__preview" title={doc?.file}>
            <PdfPreview source={{ kind: "document", id: documentId }} />
          </div>
          <div className="add-doc__hint">{text.selectHint}</div>
        </div>
        {doc && <DocumentForm doc={doc} onExclude={() => setExcluding(true)} />}
        <aside className="doc-history" aria-label={text.history}>
          <h3 className="doc-history__title">{text.history}</h3>
          <HistoryList entries={history} />
          <div className="doc-history__note">{text.historyNote}</div>
        </aside>
      </div>
      {excluding && doc && <ExcludeSheet documentId={doc.id} title={doc.title} onClose={() => setExcluding(false)} />}
    </div>
  );
}
