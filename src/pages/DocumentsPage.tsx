import { Search } from "lucide-react";
import type { DocumentsTab } from "../api";
import { useDocumentsTab } from "../documents/DocumentsTabs";
import { startAddDocument } from "../documents/addFlow";
import { InvoiceList } from "../documents/lists/InvoiceList";
import { OtherList } from "../documents/lists/OtherList";
import { PayslipList } from "../documents/lists/PayslipList";
import { StatementList } from "../documents/lists/StatementList";
import { addOtherPdf } from "../payslips/flow";
import { startImport } from "../statements/importFlow";
import { useStore } from "../state/store";
import { EmptyPage } from "./EmptyPage";
import { useT } from "../i18n";

/** Dokumente (ADR 0056): every PDF Hamster keeps, one kind at a time (the tabs sit in the toolbar). "Is it already
 *  here?" filters the open kind and is the same search as the header's filter. */
export function DocumentsPage({ tab: asked }: { tab?: DocumentsTab }) {
  const query = useStore((s) => s.filter);
  const setQuery = useStore((s) => s.setFilter);
  const tab = useDocumentsTab(asked);
  const text = useT().pages.documents;
  const noMatch = <p className="docs-page__none">{text.noMatch(query.trim())}</p>;
  const empty = (title: string, body: string, action: string, run: () => void) =>
    query.trim() ? (
      noMatch
    ) : (
      <EmptyPage title={title} text={body}>
        <button className="btn btn--primary" onClick={run}>
          {action}
        </button>
      </EmptyPage>
    );
  return (
    <div className="page-scroll">
      <div className="docs-page">
        <label className="big-search">
          <Search size={17} />
          <input placeholder={tab === "invoices" ? text.searchPlaceholder : text.searchOther} aria-label={text.searchLabel} value={query} onChange={(e) => setQuery(e.target.value)} />
        </label>
        {tab === "invoices" && <InvoiceList query={query} empty={empty(text.emptyTitle, text.emptyText, text.chooseFile, startAddDocument)} />}
        {tab === "statements" && <StatementList query={query} empty={empty(text.statements, text.emptyStatements, text.importStatement, startImport)} />}
        {tab === "payslips" && <PayslipList />}
        {tab === "other" && <OtherList query={query} empty={empty(text.other, text.emptyOther, text.addOther, addOtherPdf)} />}
      </div>
    </div>
  );
}
