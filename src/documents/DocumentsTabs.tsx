import type { DocumentsTab } from "../api";
import { useT } from "../i18n";
import { Segmented } from "../settings/controls";
import { useStore } from "../state/store";

export const DOCUMENTS_TABS: DocumentsTab[] = ["invoices", "statements", "payslips", "other"];

/** The kind of document Dokumente shows: the route's tab, else the one opened last. */
export function useDocumentsTab(asked?: DocumentsTab): DocumentsTab {
  const remembered = useStore((s) => s.documentsTab);
  return asked ?? remembered;
}

/** In the toolbar beside the title, like a macOS segmented control: one segment per kind of document. */
export function DocumentsTabs({ tab: asked }: { tab?: DocumentsTab }) {
  const tab = useDocumentsTab(asked);
  const text = useT().pages.documents;
  const choose = (next: DocumentsTab) => {
    useStore.getState().setDocumentsTab(next);
    useStore.getState().navigate({ page: "documents", tab: next });
  };
  return <Segmented label={text.tabs} options={DOCUMENTS_TABS.map((t) => ({ value: t, label: text[t] }))} value={tab} onChange={choose} />;
}
