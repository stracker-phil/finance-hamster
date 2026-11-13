import { HistoryList } from "../documents/HistoryList";
import { useYearLog } from "../queries/documents";
import { useStore } from "../state/store";
import { EmptyPage } from "./EmptyPage";

/** Every change to the year's documents, newest first; written once, never edited. */
export function LogPage({ year }: { year: number }) {
  const { data: entries } = useYearLog(year);
  const navigate = useStore((s) => s.navigate);
  if (entries?.length === 0) return <EmptyPage title="Nothing logged yet" text={`Changes to documents paid in ${year} show here.`} />;
  return (
    <div className="page-scroll">
      <div className="log-page">{entries && <HistoryList entries={entries} withDocument onOpen={(documentId) => navigate({ page: "document", documentId, year })} />}</div>
    </div>
  );
}
