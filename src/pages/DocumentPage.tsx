import { PdfPreview } from "../components/PdfPreview";

/** A document page: its PDF. */
export function DocumentPage({ documentId }: { documentId: string }) {
  return (
    <div className="document-page">
      <PdfPreview source={{ kind: "document", id: documentId }} />
    </div>
  );
}
