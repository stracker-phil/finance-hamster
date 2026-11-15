import { useState } from "react";
import { FileText } from "lucide-react";
import type { ArchivedDocument } from "../api";
import { DeleteSheet } from "../documents/DeleteSheet";
import { runStep } from "../documents/lifecycle";
import { useArchive } from "../queries/documents";
import { useSettings } from "../queries/settings";
import { EmptyPage } from "./EmptyPage";
import { useT } from "../i18n";

/** The year's excluded documents, each restored or deleted from here. */
export function ArchivePage({ year }: { year: number }) {
  const { data: docs } = useArchive(year);
  const { data: settings } = useSettings();
  const [deleting, setDeleting] = useState<ArchivedDocument | null>(null);
  const text = useT().pages.archive;
  if (!settings) return null;
  const days = settings.archiveDays;
  if (docs?.length === 0) return <EmptyPage title={text.emptyTitle} text={text.emptyText(year)} />;
  return (
    <div className="page-scroll">
      <p className="archive-page__intro">{text.intro(days)}</p>
      {docs?.map((doc) => (
        <div key={doc.id} className="archive-row">
          <FileText size={18} strokeWidth={1.8} className="docs-row__icon" />
          <div className="docs-row__text">
            <span className="mono archive-row__file">{doc.file}</span>
            <span className="add-doc__hint">
              {doc.reason} · {doc.excluded}
            </span>
          </div>
          {doc.deletableNote && <span className="add-doc__hint">{doc.deletableNote}</span>}
          <button className="btn btn--small" onClick={() => void runStep(doc.id, { kind: "restore" })}>
            {text.restore}
          </button>
          <button className="btn btn--small btn--danger-text" disabled={!doc.deletable} onClick={() => setDeleting(doc)}>
            {text.delete}
          </button>
        </div>
      ))}
      {deleting && <DeleteSheet doc={deleting} onClose={() => setDeleting(null)} />}
    </div>
  );
}
