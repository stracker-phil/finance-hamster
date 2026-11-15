import { useState } from "react";
import { Lock } from "lucide-react";
import { DetailLayout } from "../documents/DetailLayout";
import { DocumentForm } from "../documents/DocumentForm";
import { ExcludeSheet } from "../documents/ExcludeSheet";
import { HistoryList } from "../documents/HistoryList";
import { runStep, unlock } from "../documents/lifecycle";
import { useDocument, useDocumentHistory } from "../queries/documents";
import { EmptyPage } from "./EmptyPage";
import { useT } from "../i18n";

/** An invoice: its PDF over most of the page, the payment-first form and its History in the sidebar. */
export function DocumentPage({ documentId }: { documentId: string }) {
  const { data: doc, isError } = useDocument(documentId);
  const { data: history = [] } = useDocumentHistory(documentId);
  const [excluding, setExcluding] = useState(false);
  const text = useT().pages.document;
  const titles = useT().shell.titles;
  if (isError) return <EmptyPage title={text.missingTitle} text={text.missingText} />;
  const banners = (
    <>
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
    </>
  );
  return (
    <>
      <DetailLayout source={{ kind: "document", id: documentId }} banners={banners} label={doc?.title ?? titles.document} fileTitle={doc?.file}>
        {doc && <DocumentForm doc={doc} onExclude={() => setExcluding(true)} />}
        <details className="detail__history">
          <summary>{text.history}</summary>
          <HistoryList entries={history} />
          <div className="doc-history__note">{text.historyNote}</div>
        </details>
        <p className="add-doc__hint">{text.selectHint}</p>
      </DetailLayout>
      {excluding && doc && <ExcludeSheet documentId={doc.id} title={doc.title} onClose={() => setExcluding(false)} />}
    </>
  );
}
