import { PdfPreview } from "../components/PdfPreview";
import { DocumentForm } from "../documents/DocumentForm";
import { useDocument } from "../queries/documents";
import { EmptyPage } from "./EmptyPage";

/** A document: its PDF beside the payment-first form. */
export function DocumentPage({ documentId }: { documentId: string }) {
  const { data: doc, isError } = useDocument(documentId);
  if (isError) return <EmptyPage title="No such document" text="It may have been deleted, or the link is old." />;
  return (
    <div className="page-scroll">
      <div className="doc-page">
        <div className="doc-page__file">
          <div className="doc-page__preview">
            <PdfPreview source={{ kind: "document", id: documentId }} />
          </div>
          {doc && <div className="doc-page__path mono">{doc.file}</div>}
          <div className="add-doc__hint">Text in the PDF can be selected and copied</div>
        </div>
        {doc && <DocumentForm doc={doc} />}
      </div>
    </div>
  );
}
