import type { HistoryEntry, HistoryKind } from "../api";
import { useT } from "../i18n";

const KIND_CLASS: Record<HistoryKind, string> = {
  added: "history__dot--added",
  changed: "history__dot--changed",
  linked: "history__dot--linked",
  unlinked: "history__dot--changed",
  finished: "history__dot--finished",
  unlocked: "history__dot--changed",
  excluded: "history__dot--excluded",
  restored: "history__dot--added",
  deleted: "history__dot--excluded",
};

/** History lines, newest first; `withDocument` also names the document each is about. */
export function HistoryList({ entries, withDocument, onOpen }: { entries: HistoryEntry[]; withDocument?: boolean; onOpen?: (documentId: string) => void }) {
  const text = useT().documents.history;
  return (
    <ol className="history" aria-label={text.changes}>
      {entries.map((entry, i) => (
        <li key={i} className="history__entry">
          <span className={`history__dot ${KIND_CLASS[entry.kind]}`} />
          <div className="history__text">
            {withDocument &&
              (onOpen ? (
                <button className="link-btn history__doc" onClick={() => onOpen(entry.documentId)}>
                  {entry.document}
                </button>
              ) : (
                <span className="history__doc">{entry.document}</span>
              ))}
            <span>{entry.text}</span>
            <span className="history__when">{entry.when}</span>
          </div>
        </li>
      ))}
    </ol>
  );
}
