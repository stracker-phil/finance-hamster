import { useState } from "react";
import { FileText } from "lucide-react";
import type { ArchivedDocument } from "../api";
import { DeleteSheet } from "../documents/DeleteSheet";
import { runStep } from "../documents/lifecycle";
import { useArchive } from "../queries/documents";
import { useSettings } from "../queries/settings";
import { EmptyPage } from "./EmptyPage";

/** The year's excluded documents, each restored or deleted from here. */
export function ArchivePage({ year }: { year: number }) {
  const { data: docs } = useArchive(year);
  const { data: settings } = useSettings();
  const [deleting, setDeleting] = useState<ArchivedDocument | null>(null);
  if (!settings) return null;
  const days = settings.archiveDays;
  if (docs?.length === 0) return <EmptyPage title="Nothing excluded" text={`Excluded documents of ${year} show here.`} />;
  return (
    <div className="page-scroll">
      <p className="archive-page__intro">
        Excluded documents never count and never show in other views. Nothing here is removed automatically. Deleting is possible {days} {days === 1 ? "day" : "days"} after excluding, and every deletion is written to the Log.
      </p>
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
            Restore
          </button>
          <button className="btn btn--small btn--danger-text" disabled={!doc.deletable} onClick={() => setDeleting(doc)}>
            Delete…
          </button>
        </div>
      ))}
      {deleting && <DeleteSheet doc={deleting} onClose={() => setDeleting(null)} />}
    </div>
  );
}
