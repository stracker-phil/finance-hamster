import { PdfPreview } from "../components/PdfPreview";
import { DocumentForm } from "../documents/DocumentForm";
import { useDocument } from "../queries/documents";

/** A document: its PDF beside the payment-first form. */
export function DocumentPage({ documentId }: { documentId: string }) {
  const { data: doc } = useDocument(documentId);
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
