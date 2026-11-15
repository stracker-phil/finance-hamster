import { HistoryList } from "../documents/HistoryList";
import { useYearLog } from "../queries/documents";
import { useStore } from "../state/store";
import { EmptyPage } from "./EmptyPage";
import { useT } from "../i18n";

/** Every change to the year's documents, newest first; written once, never edited. */
export function LogPage({ year }: { year: number }) {
  const filter = useStore((s) => s.filter);
  const { data: entries } = useYearLog(year, filter);
  const navigate = useStore((s) => s.navigate);
  const text = useT().pages.log;
  if (entries?.length === 0) return <EmptyPage title={text.emptyTitle} text={text.emptyText(year)} />;
  return (
    <div className="page-scroll">
      <div className="log-page">{entries && <HistoryList entries={entries} withDocument onOpen={(documentId) => navigate({ page: "document", documentId, year })} />}</div>
    </div>
  );
}
